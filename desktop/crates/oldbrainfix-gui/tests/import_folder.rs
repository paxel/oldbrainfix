//! The Import of a folder driven headless (ADR 0036): File ▸ Import… ▸
//! "A folder…" (#145) asks for a folder, its Markdown and text files, also
//! in its folders but not in hidden ones, become dated Notes in a new
//! Area, and the top says it finished, what went where behind the click,
//! with a way into the Area; a folder of nothing usable and one that
//! cannot be read say why.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, TimeZone, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::imports::MAX_FILE_BYTES;
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

/// The window, its folder dialog answering `picked`.
fn window(
    store: Store,
    picked: Option<PathBuf>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store, "en");
    app.set_import_picker(Box::new(move |title| {
        assert_eq!(title, "Choose the folder to import");
        picked.clone()
    }));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    Ok(harness)
}

/// Opens File ▸ Import….
fn import_dialog(harness: &mut Harness<'_, App>) {
    harness.get_by_label("File").click();
    harness.step();
    harness.get_by_label("Import…").click();
    harness.step();
    harness.step();
}

/// Writes `text` to `path`, last changed `at`.
fn file(path: &Path, text: &str, at: DateTime<Utc>) -> TestResult {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)?;
    std::fs::File::options()
        .write(true)
        .open(path)?
        .set_modified(SystemTime::from(at))?;
    Ok(())
}

fn at(y: i32, m: u32, d: u32) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    Ok(Utc
        .with_ymd_and_hms(y, m, d, 10, 0, 0)
        .single()
        .ok_or("a date")?)
}

/// Imports the folder of `harness`'s dialog and waits for the
/// announcement.
fn import(harness: &mut Harness<'_, App>) -> TestResult {
    import_dialog(harness);
    harness.get_by_label("A folder…").click_accesskit();
    harness.step();
    step_until(harness, |app| app.notice().is_some())?;
    harness.step();
    Ok(())
}

#[test]
fn a_folder_becomes_dated_notes_in_a_new_area_and_the_click_opens_it() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Notes");
    file(&dir.join("Dentist.txt"), "Tuesday 9:00", at(2024, 3, 1)?)?;
    file(
        &dir.join("Shopping.md"),
        // Task list items become boxes (#263).
        "# Shopping\n\n- milk\n- [ ] eggs\n- [x] bread",
        at(2024, 2, 1)?,
    )?;
    file(
        &dir.join("trips").join("Rome.MD"),
        "Pack the map",
        at(2023, 5, 6)?,
    )?;
    // Hidden folders are passed over and not counted; hidden files and
    // other kinds are counted as no text, a large file as too large.
    file(
        &dir.join(".obsidian").join("workspace.md"),
        "x",
        at(2024, 1, 1)?,
    )?;
    file(&dir.join(".hidden.md"), "x", at(2024, 1, 1)?)?;
    std::fs::write(dir.join("photo.png"), [0x89, b'P', b'N', b'G'])?;
    std::fs::File::create(dir.join("huge.txt"))?.set_len(MAX_FILE_BYTES as u64 + 1)?;

    let mut harness = window(store()?, Some(dir.clone()))?;
    import_dialog(&mut harness);
    assert!(shows(
        &harness,
        "Each Markdown or text file in it becomes a Note"
    ));
    // Closed, it opens again for the Import.
    harness.get_by_label("Close").click_accesskit();
    harness.step();
    import(&mut harness)?;
    // Only that it finished; the click shows what went where.
    assert_eq!(harness.state().notice(), Some("Import finished."));
    assert!(!shows(&harness, "went into the new Area"));
    assert!(!harness.state().importing());

    let desk = harness.state().store();
    let areas = Places::new(desk).areas()?;
    let area = areas
        .iter()
        .find(|a| a.name == "From Notes")
        .ok_or("the new Area")?
        .clone();
    let notes = Notes::new(desk).all()?;
    assert_eq!(notes.len(), 3);
    assert!(notes.iter().all(|n| n.place.as_deref() == Some(&area.id)));
    assert!(Notes::new(desk).unsorted()?.is_empty(), "none is Unsorted");
    let note = |first: &str| {
        notes
            .iter()
            .find(|n| n.text.starts_with(first))
            .ok_or(format!("the Note {first}"))
    };
    let dentist = note("Dentist")?;
    assert_eq!(dentist.text, "Dentist\nTuesday 9:00");
    assert_eq!(dentist.text_format, "plain");
    assert_eq!(dentist.captured, at(2024, 3, 1)?);
    let shopping = note("# Shopping")?;
    assert_eq!(
        shopping.shown_text(),
        "# Shopping\n\n- milk\n- ☐ eggs\n- ✅ bread"
    );
    assert_eq!(shopping.text_format, "markdown");
    assert_eq!(shopping.captured, at(2024, 2, 1)?);
    assert_eq!(note("# Rome")?.captured, at(2023, 5, 6)?);

    harness.get_by_label("Import finished.").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(
        &harness,
        "3 Notes went into the new Area From Notes."
    ));
    assert!(shows(&harness, "3 files were skipped:"));
    assert!(shows(&harness, "Not Markdown or text: 2"));
    assert!(shows(&harness, "Too large: 1"));
    harness.get_by_label("Open the Area").click_accesskit();
    harness.step();
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::Place(area.id));
    assert!(!shows(&harness, "went into the new Area"));
    Ok(())
}

#[test]
fn the_same_folder_again_makes_a_second_area() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Notes");
    file(&dir.join("Dentist.txt"), "Tuesday 9:00", at(2024, 3, 1)?)?;
    let mut harness = window(store()?, Some(dir))?;
    import(&mut harness)?;
    harness.get_by_label("Import finished.").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "1 Note went into the new Area From Notes."));
    assert!(!shows(&harness, "skipped"));
    harness
        .get_all_by_label("Close")
        .last()
        .ok_or("Close")?
        .click_accesskit();
    harness.step();
    harness.step();
    import(&mut harness)?;
    let mut names: Vec<String> = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .map(|a| a.name)
        .filter(|n| n.starts_with("From Notes"))
        .collect();
    names.sort();
    assert_eq!(names, ["From Notes", "From Notes (2)"]);
    Ok(())
}

#[test]
fn a_folder_without_markdown_or_text_changes_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Photos");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("photo.png"), [0x89, b'P', b'N', b'G'])?;
    let mut harness = window(store()?, Some(dir))?;
    import(&mut harness)?;
    assert_eq!(
        harness.state().notice(),
        Some("No Markdown or text file found.")
    );
    let desk = harness.state().store();
    assert!(Places::new(desk).areas()?.is_empty());
    assert!(Notes::new(desk).all()?.is_empty());
    harness
        .get_by_label("No Markdown or text file found.")
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Nothing imported"));
    assert!(shows(&harness, "Pick the folder the notes lie in."));
    assert!(shows(&harness, "Folder: Photos"));
    assert!(shows(&harness, "Not Markdown or text: 1"));
    Ok(())
}

#[test]
fn a_folder_that_cannot_be_read_fails_and_writes_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let gone = root.path().join("Gone");
    let mut harness = window(store()?, Some(gone))?;
    import(&mut harness)?;
    assert_eq!(
        harness.state().notice(),
        Some("The folder could not be imported.")
    );
    let desk = harness.state().store();
    assert!(Places::new(desk).areas()?.is_empty());
    harness
        .get_by_label("The folder could not be imported.")
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Import failed"));
    assert!(shows(
        &harness,
        "The folder or one of its files could not be read."
    ));
    assert!(shows(
        &harness,
        "Copy the folder onto this computer's own disk"
    ));
    Ok(())
}

#[test]
fn a_cancelled_dialog_imports_nothing() -> TestResult {
    let mut harness = window(store()?, None)?;
    import_dialog(&mut harness);
    harness.get_by_label("A folder…").click_accesskit();
    for _ in 0..5 {
        harness.step();
    }
    assert!(!harness.state().importing());
    assert_eq!(harness.state().notice(), None);
    assert!(Places::new(harness.state().store()).areas()?.is_empty());
    Ok(())
}
