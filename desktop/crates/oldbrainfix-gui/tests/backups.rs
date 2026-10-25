//! Backups driven headless: set up in the Settings with a passphrase
//! typed twice, the Recovery Sheet saved, the folder chosen with a first
//! Backup at once; and the first start's view restoring one into the
//! empty log with the passphrase or the recovery key, or starting fresh.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::backup::{self, EXTENSION, Kdf, system_random};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CHEAP: Kdf = Kdf {
    memory: 64,
    iterations: 1,
    parallelism: 1,
};

fn store(seed: u8) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?)
}

fn window(app: App) -> Harness<'static, App> {
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
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// Clicks the field whose hint is `hint` and types `text` into it.
fn type_into(harness: &mut Harness<'_, App>, hint: &str, text: &str) {
    harness.get_by(|n| n.placeholder() == Some(hint)).click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// Clicks the `n`th field of `role` in the view, which holds text
/// already, and types `text` over it.
fn type_over(harness: &mut Harness<'_, App>, role: Role, n: usize, text: &str) {
    if let Some(field) = harness.query_all_by_role(role).nth(n) {
        field.click();
    }
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    harness.get_by_label(label).click_accesskit();
    harness.step();
    harness.step();
}

#[test]
fn backups_are_set_up_the_sheet_saved_and_a_folder_gets_one_at_once() -> TestResult {
    let desk = store(1)?;
    Notes::new(&desk).capture("Pills at eight", None, None)?;
    let dir = tempfile::tempdir()?;
    let mut app = App::new(desk, "en");
    let sheet = dir.path().join("sheet.pdf");
    let (saved, folder) = (sheet.clone(), dir.path().join("backups"));
    std::fs::create_dir_all(&folder)?;
    let picked = folder.clone();
    app.set_backup_dialogs(
        Box::new(move |title, name| {
            assert_eq!(title, "Save the Recovery Sheet");
            assert_eq!(name, "oldbrainfix-recovery-sheet.pdf");
            Some(saved.clone())
        }),
        Box::new(|_| None),
    );
    app.set_folder_picker(Box::new(move |title| {
        assert_eq!(title, "Choose the folder for Backups");
        Some(picked.clone())
    }));
    app.select(Selection::Settings);
    let mut harness = window(app);
    assert!(shows(&harness, "Set up Backups"));

    type_into(&mut harness, "Passphrase", "correct horse");
    type_into(&mut harness, "Passphrase again", "correct hose");
    click(&mut harness, "Set up Backups");
    assert!(shows(&harness, "The two do not match."));
    assert!(!harness.state().store().backups_set_up()?);

    type_over(&mut harness, Role::PasswordInput, 1, "correct horse");
    click(&mut harness, "Set up Backups");
    let store = harness.state().store();
    assert!(store.backups_set_up()?);
    // The sheet is offered, and saved, at once.
    let pdf = std::fs::read_to_string(&sheet)?;
    assert!(pdf.starts_with("%PDF-1.4"));
    let code = store.recovery_code()?.ok_or("a code")?;
    let first: Vec<&str> = code.split('-').take(7).collect();
    assert!(pdf.contains(&format!("({}) Tj", first.join("-"))));
    assert!(shows(&harness, "The Recovery Sheet is saved."));

    click(&mut harness, "Choose folder");
    let written: Vec<_> = std::fs::read_dir(&folder)?.collect::<Result<_, _>>()?;
    assert_eq!(written.len(), 1);
    assert!(
        written[0]
            .path()
            .extension()
            .is_some_and(|e| e == EXTENSION)
    );
    assert!(shows(&harness, "Backup written."));
    assert!(shows(&harness, "Last Backup:"));
    assert!(harness.state().store().backups_scheduled()?);

    click(&mut harness, "Back up every week");
    assert!(!harness.state().store().backups_weekly()?);
    Ok(())
}

/// A Backup of a Device with a Note in an Area, and its recovery key.
fn old_backup(
    dir: &std::path::Path,
) -> Result<(std::path::PathBuf, String), Box<dyn std::error::Error>> {
    let old = store(7)?;
    let area = Places::new(&old).add_area("Doctor")?;
    Notes::new(&old).capture("Pills at eight", Some(&area.id), None)?;
    let code = backup::set_up(&old, "correct horse", CHEAP, &mut system_random)?;
    let path = dir.join(format!("old.{EXTENSION}"));
    backup::write_backup(&old, &path, &mut system_random)?;
    Ok((path, code))
}

fn first_start(
    path: std::path::PathBuf,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store(2)?, "en");
    app.set_backup_dialogs(
        Box::new(|_, _| None),
        Box::new(move |title| {
            assert_eq!(title, "Choose the Backup file");
            Some(path.clone())
        }),
    );
    app.offer_restore();
    assert!(app.welcoming());
    Ok(window(app))
}

#[test]
fn the_first_start_restores_a_backup_with_the_passphrase() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, _) = old_backup(dir.path())?;
    let mut harness = first_start(path)?;
    assert!(shows(&harness, "Welcome to Old Brain Fix"));
    click(&mut harness, "Choose the Backup file");
    assert!(shows(&harness, "Backup of "));

    type_into(&mut harness, "Passphrase", "wrong horse");
    click(&mut harness, "Restore");
    assert!(shows(&harness, "does not open the Backup"));
    assert!(harness.state().welcoming());

    type_over(&mut harness, Role::PasswordInput, 0, "correct horse");
    click(&mut harness, "Restore");
    assert!(!harness.state().welcoming());
    assert!(shows(&harness, "Restored: "));
    let store = harness.state().store();
    let texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    assert_eq!(texts, vec!["Pills at eight"]);
    // Not offered again.
    harness.state_mut().offer_restore();
    assert!(!harness.state().welcoming());
    Ok(())
}

#[test]
fn the_first_start_restores_with_the_recovery_key_or_starts_fresh() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, code) = old_backup(dir.path())?;
    let mut harness = first_start(path.clone())?;
    click(&mut harness, "Choose the Backup file");
    type_into(&mut harness, "Recovery key", "nonsense");
    click(&mut harness, "Restore");
    assert!(shows(&harness, "That is not a recovery key."));
    type_over(&mut harness, Role::TextInput, 0, &code.to_lowercase());
    click(&mut harness, "Restore");
    assert!(!harness.state().welcoming());
    assert_eq!(Places::new(harness.state().store()).areas()?.len(), 1);

    // Starting fresh: the view goes and stays gone.
    let mut harness = first_start(path)?;
    click(&mut harness, "Start fresh");
    assert!(!harness.state().welcoming());
    harness.state_mut().offer_restore();
    assert!(!harness.state().welcoming());
    assert!(Notes::new(harness.state().store()).all()?.is_empty());
    Ok(())
}
