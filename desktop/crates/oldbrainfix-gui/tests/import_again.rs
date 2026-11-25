//! An Import of a source imported before, driven headless (obf-1078):
//! once the folder or the Google Keep export is picked, and an earlier
//! Import of it made Notes of any of its files, the window asks "Fill in
//! the Import of <day> into <Area>?". Fill in puts only the missing items
//! into that Area, and says so; with nothing missing it says "Nothing
//! new" and makes no Area. Import as new makes a new Area with every
//! item, Cancel imports nothing. A first Import asks nothing.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Area, Notes, Places, SigningKey, Store};
use oldbrainfix_gui::App;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The day of the Imports, as the question names it.
const QUESTION: &str = "Fill in the Import of Mon, 3/2/2026 into From Notes?";

/// A store whose clock stands at noon of Monday, March 2, 2026, on
/// every computer's clock.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let at = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    Ok(Store::in_memory(Box::new(move || at), || {
        Ok(SigningKey::from_seed([1; 32]))
    })?)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The window, its folder dialog answering `folder`, its zip dialog
/// `zip`.
fn window(
    folder: Option<PathBuf>,
    zip: Option<PathBuf>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store()?, "en");
    app.set_import_picker(Box::new(move |_| folder.clone()));
    app.set_import_zip_picker(Box::new(move |_| zip.clone()));
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

/// Steps the window until `done` says so, the Import's thread working
/// meanwhile.
fn step_until(
    harness: &mut Harness<'_, App>,
    done: impl Fn(&Harness<'_, App>) -> bool,
) -> TestResult {
    for _ in 0..400 {
        if done(harness) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

/// File ▸ Import… ▸ "A folder…".
fn pick_folder(harness: &mut Harness<'_, App>) {
    harness.get_by_label("File").click();
    harness.step();
    harness.get_by_label("Import…").click();
    harness.step();
    harness.step();
    harness.get_by_label("A folder…").click_accesskit();
    harness.step();
}

/// File ▸ Import… ▸ "Google Keep…" ▸ "Zip file".
fn pick_keep_zip(harness: &mut Harness<'_, App>) {
    harness.get_by_label("File").click();
    harness.step();
    harness.get_by_label("Import…").click();
    harness.step();
    harness.step();
    harness.get_by_label("Google Keep…").click_accesskit();
    harness.step();
    harness.step();
    harness.get_by_label("Zip file").click_accesskit();
    harness.step();
}

/// Waits for the Import to end, the question never asked meanwhile, and
/// closes what the Import dialog says in its place.
fn ends_unasked(harness: &mut Harness<'_, App>) -> TestResult {
    for _ in 0..400 {
        assert!(!shows(harness, "Fill in the Import of"), "asked");
        if !harness.state().importing() {
            break;
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    harness.step();
    close(harness);
    Ok(())
}

/// Waits for the question, then answers it with `answer` and waits for
/// the Import to end.
fn answers(harness: &mut Harness<'_, App>, question: &str, answer: &str) -> TestResult {
    step_until(harness, |h| shows(h, question))?;
    assert!(shows(
        harness,
        "Fill in adds there only what that Import did not bring in."
    ));
    harness.get_by_label(answer).click_accesskit();
    harness.step();
    harness.step();
    assert!(!shows(harness, question), "answered");
    step_until(harness, |h| !h.state().importing())?;
    harness.step();
    close(harness);
    Ok(())
}

/// Closes the dialog on top, if one shows Close.
fn close(harness: &mut Harness<'_, App>) {
    if let Some(close) = harness.get_all_by_label("Close").last() {
        close.click_accesskit();
    }
    harness.step();
    harness.step();
}

/// Opens what the top says behind its `notice`.
fn open_notice(harness: &mut Harness<'_, App>, notice: &str) {
    assert_eq!(harness.state().notice(), Some(notice));
    harness.get_by_label(notice).click_accesskit();
    harness.step();
    harness.step();
}

/// The Areas whose name starts with `name`.
fn areas(harness: &Harness<'_, App>, name: &str) -> Result<Vec<Area>, Box<dyn std::error::Error>> {
    let mut areas: Vec<Area> = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .filter(|a| a.name.starts_with(name))
        .collect();
    areas.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(areas)
}

/// How many Notes lie in `area`.
fn notes_in(harness: &Harness<'_, App>, area: &Area) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(Notes::new(harness.state().store()).inside(&area.id)?.len())
}

/// A folder "Notes" in `root` holding `files`, each with its text.
fn folder(root: &Path, files: &[&str]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = root.join("Notes");
    for name in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, format!("{name} text"))?;
    }
    Ok(dir)
}

/// The folder imported once, unasked.
fn imported_once(
    files: &[&str],
) -> Result<(tempfile::TempDir, PathBuf, Harness<'static, App>), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let dir = folder(root.path(), files)?;
    let mut harness = window(Some(dir.clone()), None)?;
    pick_folder(&mut harness);
    ends_unasked(&mut harness)?;
    open_notice(&mut harness, "Import finished.");
    assert!(shows(&harness, "went into the new Area From Notes."));
    close(&mut harness);
    Ok((root, dir, harness))
}

#[test]
fn a_first_import_asks_nothing_and_a_second_asks_about_it() -> TestResult {
    let (_root, _dir, mut harness) = imported_once(&["Dentist.txt", "sub/Garden.md"])?;
    assert_eq!(areas(&harness, "From Notes")?.len(), 1);
    pick_folder(&mut harness);
    step_until(&mut harness, |h| shows(h, QUESTION))?;
    // The question's three answers, nothing written while it waits.
    for answer in ["Fill in", "Import as new", "Cancel"] {
        assert!(harness.query_by_label(answer).is_some(), "{answer}");
    }
    assert!(harness.state().importing());
    harness.get_by_label("Cancel").click_accesskit();
    harness.step();
    step_until(&mut harness, |h| !h.state().importing())?;
    harness.step();
    assert!(!shows(&harness, QUESTION));
    assert_eq!(areas(&harness, "From Notes")?.len(), 1);
    Ok(())
}

#[test]
fn esc_at_the_question_imports_nothing_and_back_leaves_it_asking() -> TestResult {
    let (_root, _dir, mut harness) = imported_once(&["Dentist.txt"])?;
    pick_folder(&mut harness);
    step_until(&mut harness, |h| shows(h, QUESTION))?;
    // Back goes nowhere over a dialog: the question waits.
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.step();
    harness.step();
    assert!(shows(&harness, QUESTION));
    assert!(harness.state().importing());
    harness.key_press(egui::Key::Escape);
    harness.step();
    harness.step();
    step_until(&mut harness, |h| !h.state().importing())?;
    harness.step();
    assert!(!shows(&harness, QUESTION));
    assert_eq!(areas(&harness, "From Notes")?.len(), 1);
    assert_eq!(harness.state().notice(), None, "nothing said");
    Ok(())
}

#[test]
fn fill_in_puts_only_the_missing_files_into_the_earlier_area() -> TestResult {
    let (_root, dir, mut harness) = imported_once(&["Dentist.txt"])?;
    std::fs::write(dir.join("Garden.md"), "# Garden\n\nRoses")?;
    pick_folder(&mut harness);
    answers(&mut harness, QUESTION, "Fill in")?;
    let made = areas(&harness, "From Notes")?;
    assert_eq!(made.len(), 1, "no new Area");
    assert_eq!(notes_in(&harness, &made[0])?, 2);
    open_notice(&mut harness, "Import finished.");
    assert!(shows(&harness, "1 Note went into the Area From Notes."));
    assert!(shows(&harness, "1 was imported before and is left out."));
    assert!(!shows(&harness, "the new Area"));
    Ok(())
}

#[test]
fn import_as_new_makes_a_second_area_with_every_file() -> TestResult {
    let (_root, dir, mut harness) = imported_once(&["Dentist.txt"])?;
    std::fs::write(dir.join("Garden.md"), "# Garden\n\nRoses")?;
    pick_folder(&mut harness);
    answers(&mut harness, QUESTION, "Import as new")?;
    let made = areas(&harness, "From Notes")?;
    assert_eq!(made.len(), 2);
    assert_eq!(made[1].name, "From Notes (2)");
    assert_eq!(notes_in(&harness, &made[0])?, 1);
    assert_eq!(notes_in(&harness, &made[1])?, 2);
    open_notice(&mut harness, "Import finished.");
    assert!(shows(
        &harness,
        "2 Notes went into the new Area From Notes (2)."
    ));
    Ok(())
}

#[test]
fn with_nothing_missing_it_says_nothing_new_and_makes_no_area() -> TestResult {
    let (_root, _dir, mut harness) = imported_once(&["Dentist.txt", "Garden.md"])?;
    pick_folder(&mut harness);
    answers(&mut harness, QUESTION, "Fill in")?;
    open_notice(&mut harness, "Nothing new to import.");
    assert!(shows(&harness, "Nothing new: 2 already imported."));
    let made = areas(&harness, "From Notes")?;
    assert_eq!(made.len(), 1);
    assert_eq!(notes_in(&harness, &made[0])?, 2);
    Ok(())
}

#[test]
fn a_keep_zip_imported_again_is_asked_about_and_filled_in() -> TestResult {
    let root = tempfile::tempdir()?;
    let zip = root.path().join("takeout.zip");
    let note = |title: &str| {
        format!(
            r#"{{"isTrashed":false,"title":"{title}","textContent":"text","createdTimestampUsec":1700000000000000}}"#
        )
    };
    let write = |titles: &[&str]| -> TestResult {
        let mut out = ZipWriter::new(std::fs::File::create(&zip)?);
        for title in titles {
            out.start_file(
                format!("Takeout/Keep/{title}.json"),
                SimpleFileOptions::default(),
            )?;
            std::io::Write::write_all(&mut out, note(title).as_bytes())?;
        }
        out.finish()?;
        Ok(())
    };
    write(&["Dentist"])?;
    let mut harness = window(None, Some(zip.clone()))?;
    pick_keep_zip(&mut harness);
    ends_unasked(&mut harness)?;
    assert_eq!(areas(&harness, "From Google Keep")?.len(), 1);
    write(&["Dentist", "Groceries"])?;
    pick_keep_zip(&mut harness);
    answers(
        &mut harness,
        "Fill in the Import of Mon, 3/2/2026 into From Google Keep?",
        "Fill in",
    )?;
    let made = areas(&harness, "From Google Keep")?;
    assert_eq!(made.len(), 1, "no new Area");
    assert_eq!(notes_in(&harness, &made[0])?, 2);
    Ok(())
}
