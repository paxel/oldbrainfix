//! The import from Google Keep driven headless (ADR 0036, Google Keep):
//! "Import from Google Keep…" in the Sync view asks whether the Takeout
//! export is a zip or a folder, the system's dialog picks it, and its
//! Keep notes become Notes in the new Area "From Google Keep": dated when
//! made in Keep, labels as #tags, a list as a Checklist with its checked
//! Lines that stays off the Agenda, photos as photos, the trash skipped.
//! The top says it finished, what went where behind the click, with a
//! way into the Area; an export of nothing usable and one that cannot be
//! read say why.

use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::imports::MAX_FILE_BYTES;
use oldbrainfix_core::{Checklists, Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An entry of a Takeout: its path and its bytes.
type Entry = (String, Vec<u8>);

/// The button in the Sync view.
const OPEN: &str = "Import from Google Keep…";

fn store() -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([1; 32]))
    })?)
}

fn step_until(harness: &mut Harness<'_, App>, done: impl Fn(&App) -> bool) -> TestResult {
    for _ in 0..400 {
        if done(harness.state()) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The Sync view, its zip dialog answering `zip` and its folder dialog
/// `folder`.
fn sync_view(
    store: Store,
    zip: Option<PathBuf>,
    folder: Option<PathBuf>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    app.set_import_zip_picker(Box::new(move |title| {
        assert_eq!(title, "Choose the Takeout zip file");
        zip.clone()
    }));
    app.set_import_picker(Box::new(move |title| {
        assert_eq!(title, "Choose the folder unpacked from the Takeout zip");
        folder.clone()
    }));
    app.select(Selection::Sync);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    step_until(&mut harness, |app| app.sync_host().hosting())?;
    harness.step();
    Ok(harness)
}

fn at(y: i32, m: u32, d: u32) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    Ok(Utc
        .with_ymd_and_hms(y, m, d, 10, 0, 0)
        .single()
        .ok_or("a date")?)
}

fn usec(at: DateTime<Utc>) -> i64 {
    at.timestamp_micros()
}

/// A PNG of one transparent pixel.
const PNG: [u8; 67] = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

/// A small Takeout of Keep, by path: a note with labels, a photo, one
/// missing and a voice recording; a list with a checked Line; a note in
/// the trash, one that is no JSON, one of labels alone, one too large;
/// the `.html` twin and `Labels.txt`, which are not read; and a photo
/// of another Google product.
fn takeout() -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let dentist = format!(
        concat!(
            r#"{{"isTrashed":false,"isPinned":true,"isArchived":false,"#,
            r#""title":"Dentist","textContent":"Tuesday 9:00","#,
            r#""createdTimestampUsec":{},"userEditedTimestampUsec":{},"#,
            r#""labels":[{{"name":"Errands"}},{{"name":"Home Garden"}}],"#,
            r#""attachments":[{{"filePath":"1a2b3c.png","mimetype":"image/png"}},"#,
            r#"{{"filePath":"gone.jpg","mimetype":"image/jpeg"}},"#,
            r#"{{"filePath":"7a8b9c.3gp","mimetype":"audio/3gpp"}}]}}"#
        ),
        usec(at(2024, 3, 1)?),
        usec(at(2024, 6, 2)?),
    );
    let groceries = format!(
        concat!(
            r#"{{"isTrashed":false,"isArchived":true,"title":"Groceries","#,
            r#""createdTimestampUsec":{},"userEditedTimestampUsec":{},"#,
            r#""listContent":[{{"text":"Milk","isChecked":true}},"#,
            r#"{{"text":"Eggs","isChecked":false}}]}}"#
        ),
        usec(at(2024, 11, 5)?),
        usec(at(2025, 1, 10)?),
    );
    let trashed = format!(
        r#"{{"isTrashed":true,"title":"Old idea","textContent":"gone","createdTimestampUsec":{}}}"#,
        usec(at(2023, 8, 15)?),
    );
    let labels_only = format!(
        r#"{{"labels":[{{"name":"Errands"}}],"createdTimestampUsec":{}}}"#,
        usec(at(2023, 8, 15)?),
    );
    let text = |path: &str, text: &str| (path.to_string(), text.as_bytes().to_vec());
    Ok(vec![
        text("Takeout/archive_browser.html", "<html></html>"),
        text("Takeout/Keep/Labels.txt", "Errands\nHome Garden\n"),
        text("Takeout/Keep/Dentist.json", &dentist),
        text("Takeout/Keep/Dentist.html", "<html>Dentist</html>"),
        ("Takeout/Keep/1a2b3c.png".to_string(), PNG.to_vec()),
        ("Takeout/Keep/7a8b9c.3gp".to_string(), vec![0, 0, 0, 0x18]),
        text("Takeout/Keep/Groceries.json", &groceries),
        text("Takeout/Keep/Old idea.json", &trashed),
        text("Takeout/Keep/Broken.json", r#"{"title":"Broken","#),
        text("Takeout/Keep/Labels only.json", &labels_only),
        (
            "Takeout/Keep/Huge.json".to_string(),
            vec![b' '; MAX_FILE_BYTES + 1],
        ),
        ("Takeout/Google Photos/Trip.png".to_string(), PNG.to_vec()),
    ])
}

/// The Takeout as its zip at `path`, compressed as Google compresses it.
fn zip_of(path: &Path, entries: &[Entry]) -> TestResult {
    let mut zip = ZipWriter::new(std::fs::File::create(path)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in entries {
        zip.start_file(name.as_str(), options)?;
        std::io::Write::write_all(&mut zip, bytes)?;
    }
    zip.finish()?;
    Ok(())
}

/// The Takeout unpacked into `dir`.
fn folder_of(dir: &Path, entries: &[Entry]) -> TestResult {
    for (name, bytes) in entries {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
    }
    Ok(())
}

/// Asks for the import from Google Keep, answers `answer` ("Zip file" or
/// "Folder") and waits for the announcement.
fn import(harness: &mut Harness<'_, App>, answer: &str) -> TestResult {
    harness.get_by_label(OPEN).click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(
        harness,
        "Is the Takeout export a zip file, or a folder unpacked from it?"
    ));
    harness.get_by_label(answer).click_accesskit();
    harness.step();
    step_until(harness, |app| app.notice().is_some())?;
    harness.step();
    Ok(())
}

/// What the import of [`takeout`] made in `harness`'s store, as the
/// export says; returns the new Area's id.
fn made_as_keep_says(harness: &Harness<'_, App>) -> Result<String, Box<dyn std::error::Error>> {
    let desk = harness.state().store();
    let areas = Places::new(desk).areas()?;
    assert_eq!(areas.len(), 1);
    let area = areas
        .iter()
        .find(|a| a.name == "From Google Keep")
        .ok_or("the new Area")?;
    let notes = Notes::new(desk).all()?;
    assert_eq!(
        notes.len(),
        2,
        "the trash, no JSON and labels alone skipped"
    );
    assert!(notes.iter().all(|n| n.place.as_deref() == Some(&area.id)));
    assert!(Notes::new(desk).unsorted()?.is_empty(), "none is Unsorted");
    let note = |first: &str| {
        notes
            .iter()
            .find(|n| n.text.starts_with(first))
            .ok_or(format!("the Note {first}"))
    };
    let dentist = note("Dentist")?;
    assert_eq!(dentist.text, "Dentist\nTuesday 9:00\n#Errands #Home-Garden");
    assert_eq!(dentist.text_format, "plain");
    assert_eq!(dentist.captured, at(2024, 3, 1)?);
    assert_eq!(dentist.photos.len(), 1, "the PNG, not the trip's");
    let groceries = note("Groceries")?;
    assert_eq!(groceries.captured, at(2024, 11, 5)?);
    let checklists = Checklists::new(desk);
    let list = checklists.by_note(&groceries.id)?.ok_or("a Checklist")?;
    let lines: Vec<(&str, bool)> = list
        .lines
        .iter()
        .map(|l| (l.text.as_str(), l.checked))
        .collect();
    assert_eq!(lines, [("Milk", true), ("Eggs", false)]);
    assert!(!checklists.shows_on_agenda(&groceries.id)?);
    Ok(area.id.clone())
}

/// The details behind "Import finished." for [`takeout`].
fn says_what_went_where(harness: &mut Harness<'_, App>) -> TestResult {
    harness.get_by_label("Import finished.").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(
        harness,
        "2 Notes went into the new Area From Google Keep."
    ));
    assert!(shows(
        harness,
        "1 of them is a Checklist. Checklists stay off the Agenda until you switch them on."
    ));
    assert!(shows(harness, "1 photo went into the Area with them."));
    assert!(shows(harness, "6 notes or files were skipped:"));
    assert!(shows(harness, "In Keep's trash: 1"));
    assert!(shows(harness, "Missing attachment: 1"));
    assert!(shows(harness, "Not a photo: 1"));
    assert!(shows(harness, "Too large: 1"));
    assert!(shows(harness, "Not text: 2"));
    Ok(())
}

#[test]
fn a_takeout_zip_becomes_notes_in_from_google_keep_and_the_click_opens_it() -> TestResult {
    let root = tempfile::tempdir()?;
    let zip = root.path().join("takeout-20261101T000000Z-001.zip");
    zip_of(&zip, &takeout()?)?;
    let mut harness = sync_view(store()?, Some(zip), None)?;
    assert!(shows(
        &harness,
        "The notes, lists and photos of a Google Takeout export become Notes"
    ));
    import(&mut harness, "Zip file")?;
    // Only that it finished; the click shows what went where.
    assert_eq!(harness.state().notice(), Some("Import finished."));
    assert!(!shows(&harness, "went into the new Area"));
    assert!(!harness.state().importing());
    let area = made_as_keep_says(&harness)?;

    says_what_went_where(&mut harness)?;
    harness.get_by_label("Open the Area").click_accesskit();
    harness.step();
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::Place(area));
    assert!(!shows(&harness, "went into the new Area"));
    Ok(())
}

#[test]
fn an_unpacked_takeout_folder_imports_the_same() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Takeout export");
    folder_of(&dir, &takeout()?)?;
    // A photo of another product is never read: this one would fail the
    // Import.
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        root.path().join("nowhere.jpg"),
        dir.join("Takeout/Google Photos/Gone.jpg"),
    )?;
    let mut harness = sync_view(store()?, None, Some(dir))?;
    import(&mut harness, "Folder")?;
    assert_eq!(harness.state().notice(), Some("Import finished."));
    made_as_keep_says(&harness)?;
    says_what_went_where(&mut harness)
}

#[test]
fn an_imported_checklist_stays_off_the_agenda_until_switched_on() -> TestResult {
    let root = tempfile::tempdir()?;
    let zip = root.path().join("takeout.zip");
    zip_of(&zip, &takeout()?)?;
    let mut harness = sync_view(store()?, Some(zip), None)?;
    import(&mut harness, "Zip file")?;
    harness.state_mut().select(Selection::Agenda);
    harness.step();
    harness.step();
    assert!(!shows(&harness, "Groceries: 1 of 2 open"));

    let desk = harness.state().store();
    let groceries = Notes::new(desk)
        .all()?
        .into_iter()
        .find(|n| n.text.starts_with("Groceries"))
        .ok_or("the list")?;
    Checklists::new(desk).set_show_on_agenda(&groceries.id, true)?;
    harness.state_mut().select(Selection::All);
    harness.step();
    harness.state_mut().select(Selection::Agenda);
    harness.step();
    harness.step();
    assert!(shows(&harness, "Groceries: 1 of 2 open"));
    Ok(())
}

#[test]
fn a_name_leaving_the_zip_is_refused_and_counted() -> TestResult {
    let root = tempfile::tempdir()?;
    let zip = root.path().join("takeout.zip");
    let note = format!(
        r#"{{"title":"Kept","createdTimestampUsec":{}}}"#,
        usec(at(2024, 3, 1)?)
    );
    let evil = format!(
        r#"{{"title":"Evil","createdTimestampUsec":{}}}"#,
        usec(at(2024, 3, 1)?)
    );
    zip_of(
        &zip,
        &[
            ("Takeout/Keep/Kept.json".to_string(), note.into_bytes()),
            ("Takeout/../Keep/Evil.json".to_string(), evil.into_bytes()),
        ],
    )?;
    let mut harness = sync_view(store()?, Some(zip), None)?;
    import(&mut harness, "Zip file")?;
    let notes = Notes::new(harness.state().store()).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Kept");
    harness.get_by_label("Import finished.").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "1 note or file was skipped:"));
    assert!(shows(&harness, "Not text: 1"));
    Ok(())
}

#[test]
fn an_export_without_a_keep_note_changes_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let zip = root.path().join("takeout.zip");
    zip_of(
        &zip,
        &[
            (
                "Takeout/Keep/Dentist.html".to_string(),
                b"<html></html>".to_vec(),
            ),
            ("Takeout/Keep/Labels.txt".to_string(), b"Errands\n".to_vec()),
            (
                "Takeout/Keep/Old idea.json".to_string(),
                format!(
                    r#"{{"isTrashed":true,"title":"Old","createdTimestampUsec":{}}}"#,
                    usec(at(2023, 8, 15)?)
                )
                .into_bytes(),
            ),
        ],
    )?;
    let mut harness = sync_view(store()?, Some(zip), None)?;
    import(&mut harness, "Zip file")?;
    assert_eq!(harness.state().notice(), Some("No Keep note found."));
    let desk = harness.state().store();
    assert!(Places::new(desk).areas()?.is_empty());
    assert!(Notes::new(desk).all()?.is_empty());
    harness
        .get_by_label("No Keep note found.")
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Nothing imported"));
    assert!(shows(
        &harness,
        "The export holds no note of Google Keep that could be taken"
    ));
    assert!(shows(&harness, "Pick the Takeout zip file"));
    assert!(shows(&harness, "In Keep's trash: 1"));
    Ok(())
}

#[test]
fn a_zip_that_cannot_be_read_fails_and_writes_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let zip = root.path().join("takeout.zip");
    std::fs::write(&zip, b"not a zip at all")?;
    let mut harness = sync_view(store()?, Some(zip), None)?;
    import(&mut harness, "Zip file")?;
    assert_eq!(
        harness.state().notice(),
        Some("The export could not be imported.")
    );
    assert!(Places::new(harness.state().store()).areas()?.is_empty());
    harness
        .get_by_label("The export could not be imported.")
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Import failed"));
    assert!(shows(
        &harness,
        "The zip file or the folder, or one of the files in it, could not be read."
    ));
    assert!(shows(&harness, "copy it onto this computer's own disk"));
    Ok(())
}

#[test]
fn cancel_or_a_cancelled_dialog_imports_nothing() -> TestResult {
    let mut harness = sync_view(store()?, None, None)?;
    harness.get_by_label(OPEN).click_accesskit();
    harness.step();
    harness.step();
    harness.get_by_label("Cancel").click_accesskit();
    harness.step();
    harness.step();
    assert!(!shows(&harness, "Is the Takeout export a zip file"));
    // The system's dialog closed without a pick.
    import_cancelled(&mut harness, "Zip file")?;
    import_cancelled(&mut harness, "Folder")?;
    assert!(Places::new(harness.state().store()).areas()?.is_empty());
    Ok(())
}

/// Answers `answer` in the dialog, whose system dialog picks nothing.
fn import_cancelled(harness: &mut Harness<'_, App>, answer: &str) -> TestResult {
    harness.get_by_label(OPEN).click_accesskit();
    harness.step();
    harness.step();
    harness.get_by_label(answer).click_accesskit();
    for _ in 0..5 {
        harness.step();
    }
    assert!(!shows(harness, "Is the Takeout export a zip file"));
    assert!(!harness.state().importing());
    assert_eq!(harness.state().notice(), None);
    Ok(())
}
