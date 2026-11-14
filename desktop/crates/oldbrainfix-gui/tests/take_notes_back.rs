//! Taking Notes back from a Backup in the window (#285, Q682 (a)): "Open
//! a Backup…" on the Backups dialog, set up or not, and in the File menu
//! asks for the file in the Backups folder, opens it with the passphrase
//! or the recovery key on a page of its own and refuses what restoring
//! refuses, announced at the top; then the same page shows its Notes as
//! a Pack's tree under its date, the ones from its Trash said to be
//! there, none ticked but everything under each, and Add brings the
//! ticked ones back as copies in new Areas, announced, the page closed.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{AccessKitNode, NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::backup::{self, EXTENSION, Kdf, system_random};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Page};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CHEAP: Kdf = Kdf {
    memory: 64,
    iterations: 1,
    parallelism: 1,
};

/// The photo of "Knee exercises", as large as a photo may be in a test.
const PHOTO: [u8; 3000] = [7u8; 3000];

fn store(seed: u8) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?)
}

/// A Backup in `dir` of a Device with its Doctor: "Pills at eight",
/// "Knee exercises" with a photo and "Old prescription" in the trash;
/// "Call the plumber" filed nowhere. Its recovery key.
fn old_backup(dir: &Path) -> Result<(PathBuf, String), Box<dyn std::error::Error>> {
    let old = store(7)?;
    let doctor = Places::new(&old).add_area("Doctor")?;
    let notes = Notes::new(&old);
    notes.capture("Pills at eight", Some(&doctor.id), None)?;
    let knee = notes.capture("Knee exercises", Some(&doctor.id), None)?;
    notes.add_photo(&knee.id, &PHOTO)?;
    let old_note = notes.capture("Old prescription", Some(&doctor.id), None)?;
    notes.trash(&old_note.id)?;
    notes.capture("Call the plumber", None, None)?;
    let code = backup::set_up(&old, "correct horse", CHEAP, &mut system_random)?;
    let path = dir.join(format!("old.{EXTENSION}"));
    backup::write_backup(&old, &path, &mut system_random)?;
    Ok((path, code))
}

/// The folders the file dialog started in.
type Started = Arc<Mutex<Vec<Option<PathBuf>>>>;

/// This Device, with a Doctor and a Note of its own and its Backups
/// folder `folder`, the file dialog picking `path`.
fn desk(folder: &Path, path: PathBuf) -> Result<(App, Started), Box<dyn std::error::Error>> {
    let desk = store(2)?;
    let doctor = Places::new(&desk).add_area("Doctor")?;
    Notes::new(&desk).capture("Water the plants", Some(&doctor.id), None)?;
    desk.set_backup_folder(Some(&folder.to_string_lossy()))?;
    let mut app = App::new(desk, "en");
    let started = Started::default();
    let record = started.clone();
    app.set_backup_notes_opener(Box::new(move |title, folder| {
        assert_eq!(title, "Choose the Backup file");
        if let Ok(mut started) = record.lock() {
            started.push(folder.map(Path::to_path_buf));
        }
        Some(path.clone())
    }));
    Ok((app, started))
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
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

/// The last widget labelled `label`: on the page over what lies under it.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    last(harness, label).click_accesskit();
    harness.run();
}

/// Clicks the empty field whose hint is `hint` and types `text` into it.
fn type_into(harness: &mut Harness<'_, App>, hint: &str, text: &str) {
    harness.get_by(|n| n.placeholder() == Some(hint)).click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// Clicks the field `field` finds, which holds text, and types `text`
/// over it.
fn type_over(
    harness: &mut Harness<'_, App>,
    field: impl Fn(&AccessKitNode<'_>) -> bool,
    text: &str,
) {
    harness.get_by(|n| field(n)).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// The tree's box `label`.
fn tick<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    harness.get_by_role_and_label(Role::CheckBox, label)
}

fn ticked(harness: &Harness<'_, App>, label: &str) -> bool {
    tick(harness, label).accesskit_node().toggled() == Some(Toggled::True)
}

/// The label of the tree's box that starts with `prefix`.
fn starting(harness: &Harness<'_, App>, prefix: &str) -> Result<String, String> {
    harness
        .query_all_by_role(Role::CheckBox)
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| l.starts_with(prefix))
        .ok_or(format!("no {prefix}"))
}

fn add_enabled(harness: &Harness<'_, App>) -> bool {
    !last(harness, "Add").accesskit_node().is_disabled()
}

/// The texts of the Notes in the Area named `name`, none when there is
/// no such Area.
fn texts_in(store: &Store, name: &str) -> Result<Option<Vec<String>>, Box<dyn std::error::Error>> {
    let Some(area) = Places::new(store)
        .areas()?
        .into_iter()
        .find(|a| a.name == name)
    else {
        return Ok(None);
    };
    let mut texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .filter(|n| n.place.as_deref() == Some(area.id.as_str()))
        .map(|n| n.text)
        .collect();
    texts.sort();
    Ok(Some(texts))
}

#[test]
fn a_backup_opened_from_the_backups_dialog_gives_back_the_notes_ticked() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, _) = old_backup(dir.path())?;
    let (mut app, started) = desk(dir.path(), path)?;
    app.open(Page::Backups);
    let mut harness = window(app);
    // Not set up here, and offered all the same.
    assert!(shows(&harness, "Set up Backups"));
    assert!(shows(&harness, "Take Notes back"));
    click(&mut harness, "Open a Backup…");
    assert_eq!(
        *started.lock().unwrap(),
        vec![Some(dir.path().to_path_buf())]
    );
    assert_eq!(harness.state().page(), Some(Page::BackupNotes));
    // A way back to the Backups dialog.
    assert!(shows(&harness, "Back to Backups"));
    assert!(shows(&harness, "Backup of "));

    // A wrong passphrase is announced; the page stays for another try.
    type_into(&mut harness, "Passphrase", "wrong horse");
    click(&mut harness, "Open");
    assert_eq!(harness.state().notice(), Some("The Backup was not opened."));
    assert_eq!(harness.state().page(), Some(Page::BackupNotes));
    assert!(!shows(&harness, "Pills at eight"));
    click(&mut harness, "The Backup was not opened.");
    assert!(shows(
        &harness,
        "This passphrase or recovery key does not open the Backup."
    ));
    click(&mut harness, "Close");

    type_over(
        &mut harness,
        |n| n.role() == Role::PasswordInput,
        "correct horse",
    );
    click(&mut harness, "Open");
    assert_eq!(harness.state().page(), Some(Page::BackupNotes));
    // Its Notes, none ticked, the one from its Trash said to be there.
    for note in [
        "Pills at eight",
        "Knee exercises",
        "Old prescription",
        "Call the plumber",
    ] {
        assert!(!ticked(&harness, note), "{note}");
    }
    assert!(shows(&harness, "In the Trash"));
    assert_eq!(harness.query_all_by_label("In the Trash").count(), 1);
    assert!(!add_enabled(&harness));
    assert!(shows(&harness, "Tick at least one Note"));

    tick(&harness, "Pills at eight").click();
    harness.run();
    tick(&harness, "Old prescription").click();
    harness.run();
    tick(&harness, "Call the plumber").click();
    harness.run();
    assert!(add_enabled(&harness));
    click(&mut harness, "Add");
    assert_eq!(harness.state().notice(), Some("3 Notes added"));
    // Back on the Backups dialog.
    assert_eq!(harness.state().page(), Some(Page::Backups));
    let store = harness.state().store();
    // Its own Doctor as it was, the copies beside it, out of the trash.
    assert_eq!(
        texts_in(store, "Doctor")?,
        Some(vec!["Water the plants".to_string()])
    );
    assert_eq!(
        texts_in(store, "Doctor (Backup)")?,
        Some(vec![
            "Old prescription".to_string(),
            "Pills at eight".to_string()
        ])
    );
    assert_eq!(
        texts_in(store, "From a Backup")?,
        Some(vec!["Call the plumber".to_string()])
    );
    assert!(Notes::new(store).trashed()?.is_empty());
    Ok(())
}

#[test]
fn untick_all_goes_back_to_the_start_so_a_note_comes_back_with_its_photo() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, _) = old_backup(dir.path())?;
    let (mut app, _) = desk(dir.path(), path)?;
    app.open(Page::Backups);
    let mut harness = window(app);
    click(&mut harness, "Open a Backup…");
    type_into(&mut harness, "Passphrase", "correct horse");
    click(&mut harness, "Open");
    let photo = starting(&harness, "Photo")?;
    // Everything under each Note ticked from the start.
    assert!(ticked(&harness, &photo));
    click(&mut harness, "Tick all");
    assert!(ticked(&harness, "Knee exercises"));
    click(&mut harness, "Untick all");
    assert!(!ticked(&harness, "Knee exercises"));
    assert!(!ticked(&harness, "Pills at eight"));
    tick(&harness, "Knee exercises").click();
    harness.run();
    assert!(ticked(&harness, &photo));
    click(&mut harness, "Add");
    assert_eq!(harness.state().notice(), Some("1 Note added"));
    let store = harness.state().store();
    let knee = Notes::new(store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Knee exercises")
        .ok_or("the copy")?;
    assert_eq!(knee.photos.len(), 1);
    let bytes = Notes::new(store).photo_bytes(&knee.photos[0]);
    assert_eq!(bytes.as_deref(), Some(&PHOTO[..]));
    Ok(())
}

#[test]
fn the_file_menu_opens_a_backup_with_the_recovery_key() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, code) = old_backup(dir.path())?;
    let (app, _) = desk(dir.path(), path)?;
    let mut harness = window(app);
    harness
        .get_all_by_label("File")
        .next()
        .ok_or("File")?
        .click();
    harness.run();
    click(&mut harness, "Open a Backup…");
    // Alone, with no way back to a dialog it was not opened from.
    assert_eq!(harness.state().page(), Some(Page::BackupNotes));
    assert!(!shows(&harness, "Back to Backups"));

    type_into(&mut harness, "Recovery key", "nonsense");
    click(&mut harness, "Open");
    assert!(shows(&harness, "That is not a recovery key."));
    assert!(harness.state().notice().is_none());
    type_over(
        &mut harness,
        |n| n.role() == Role::TextInput && n.value().as_deref() == Some("nonsense"),
        &code.to_lowercase(),
    );
    click(&mut harness, "Open");
    assert!(!shows(&harness, "That is not a recovery key."));
    assert!(shows(&harness, "Pills at eight"));
    // What opened it is gone from the page.
    assert!(
        harness
            .query_all_by_value(&code.to_lowercase())
            .next()
            .is_none()
    );

    // Closed, nothing is added.
    harness.state_mut().close_page();
    harness.run();
    assert_eq!(harness.state().page(), None);
    assert_eq!(texts_in(harness.state().store(), "Doctor (Backup)")?, None);
    Ok(())
}

#[test]
fn a_file_that_is_no_backup_or_of_a_later_format_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let none = dir.path().join("notes.txt");
    std::fs::write(&none, b"Pills at eight\n")?;
    let (mut app, _) = desk(dir.path(), none)?;
    app.open(Page::Backups);
    let mut harness = window(app);
    click(&mut harness, "Open a Backup…");
    assert_eq!(harness.state().page(), Some(Page::Backups));
    assert_eq!(harness.state().notice(), Some("The Backup was not opened."));
    click(&mut harness, "The Backup was not opened.");
    assert!(shows(&harness, "Backup not opened"));
    assert!(shows(&harness, "This file is not a Backup"));

    let later = dir.path().join(format!("later.{EXTENSION}"));
    std::fs::write(&later, b"{\"format\":2,\"type\":\"oldbrainfix-backup\"}\n")?;
    let (mut app, _) = desk(dir.path(), later)?;
    app.open(Page::Backups);
    let mut harness = window(app);
    click(&mut harness, "Open a Backup…");
    assert_eq!(harness.state().page(), Some(Page::Backups));
    click(&mut harness, "The Backup was not opened.");
    assert!(shows(
        &harness,
        "This Backup was made by a newer Old Brain Fix."
    ));
    Ok(())
}

#[test]
fn a_backup_changed_after_it_was_opened_adds_nothing_and_stays_open() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, _) = old_backup(dir.path())?;
    let (mut app, _) = desk(dir.path(), path.clone())?;
    app.open(Page::Backups);
    let mut harness = window(app);
    click(&mut harness, "Open a Backup…");
    type_into(&mut harness, "Passphrase", "correct horse");
    click(&mut harness, "Open");
    tick(&harness, "Knee exercises").click();
    harness.run();
    // Cut off after opening: its photo cannot be read again.
    let bytes = std::fs::read(&path)?;
    std::fs::write(&path, &bytes[..bytes.len() / 2])?;
    let before = harness.state().store().current_seq()?;
    click(&mut harness, "Add");
    assert_eq!(harness.state().notice(), Some("The Notes were not added."));
    assert_eq!(harness.state().page(), Some(Page::BackupNotes));
    assert_eq!(harness.state().store().current_seq()?, before);
    assert!(ticked(&harness, "Knee exercises"));
    click(&mut harness, "The Notes were not added.");
    assert!(shows(&harness, "Nothing was added; open the Backup again."));
    Ok(())
}

#[test]
fn a_damaged_backup_is_refused_on_opening_and_an_older_one_is_asked_for() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, _) = old_backup(dir.path())?;
    let (mut app, _) = desk(dir.path(), path.clone())?;
    app.open(Page::Backups);
    let mut harness = window(app);
    click(&mut harness, "Open a Backup…");
    // Cut off after its header was read.
    let bytes = std::fs::read(&path)?;
    std::fs::write(&path, &bytes[..bytes.len() / 2])?;
    type_into(&mut harness, "Passphrase", "correct horse");
    click(&mut harness, "Open");
    assert_eq!(harness.state().notice(), Some("The Backup was not opened."));
    assert_eq!(harness.state().page(), Some(Page::BackupNotes));
    assert!(!shows(&harness, "Pills at eight"));
    click(&mut harness, "The Backup was not opened.");
    assert!(shows(&harness, "Nothing was changed; try an older Backup."));
    Ok(())
}

/// Draws frames until `done`, while the work runs on its thread.
fn until(harness: &mut Harness<'_, App>, done: impl Fn(&Harness<'_, App>) -> bool) {
    for _ in 0..500 {
        harness.step();
        if done(harness) {
            harness.run();
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the work on its thread did not arrive");
}

#[test]
fn a_backup_is_opened_and_added_off_the_window_thread() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, _) = old_backup(dir.path())?;
    let db = dir.path().join("desk.db");
    let open = |db: &Path| {
        Store::open(db, Box::new(chrono::Utc::now), || {
            Ok(SigningKey::from_seed([2; 32]))
        })
    };
    let mut app = App::new(open(&db)?, "en");
    let aside = db.clone();
    app.set_agenda_store(move || Arc::new(open(&aside).expect("a connection")));
    app.set_backup_notes_opener(Box::new(move |_, _| Some(path.clone())));
    app.open(Page::Backups);
    let mut harness = window(app);
    click(&mut harness, "Open a Backup…");
    type_into(&mut harness, "Passphrase", "correct horse");
    last(&harness, "Open").click_accesskit();
    until(&mut harness, |h| shows(h, "In the Trash"));
    tick(&harness, "Pills at eight").click();
    harness.run();
    last(&harness, "Add").click_accesskit();
    until(&mut harness, |h| h.state().notice().is_some());
    assert_eq!(harness.state().notice(), Some("1 Note added"));
    assert_eq!(harness.state().page(), Some(Page::Backups));
    // Written on the thread's connection, read on the window's.
    assert_eq!(
        texts_in(harness.state().store(), "Doctor")?,
        Some(vec!["Pills at eight".to_string()])
    );
    Ok(())
}
