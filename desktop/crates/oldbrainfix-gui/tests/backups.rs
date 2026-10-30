//! Backups driven headless: set up in the Settings with a passphrase
//! typed twice, the Recovery Sheet saved, the folder chosen with a first
//! Backup at once; and the first start's view restoring one into the
//! empty log with the passphrase or the recovery key, or starting new.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::cell::RefCell;
use std::rc::Rc;

use oldbrainfix_core::backup::{self, EXTENSION, Kdf, system_random};
use oldbrainfix_core::{DeviceSettings, Notes, Places, SigningKey, Store};
use oldbrainfix_gui::media::Player;
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

/// The own Profile's name, none before there is one.
fn own_name(harness: &Harness<'_, App>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let own = oldbrainfix_core::profile::OwnProfile::new(harness.state().store()).get()?;
    Ok(own.map(|p| p.name))
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
    app.select(Selection::Profile);
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
    // Shown as the last Backup: no announcement.
    assert!(!shows(&harness, "Backup written."));
    assert!(shows(&harness, "Last Backup:"));
    assert!(harness.state().store().backups_scheduled()?);

    click(&mut harness, "Back up every week");
    assert!(!harness.state().store().backups_weekly()?);
    Ok(())
}

#[test]
fn backups_set_up_on_a_paired_phone_show_with_the_same_recovery_sheet() -> TestResult {
    use oldbrainfix_core::lan::{answer_sync, apply_answer, sync_request};
    use oldbrainfix_core::profile::OwnProfile;
    let phone = store(2)?;
    let desk = store(1)?;
    OwnProfile::new(&phone).apply_defaults("Pixel", None)?;
    OwnProfile::new(&phone).rename("Ada")?;
    OwnProfile::new(&desk).apply_defaults("Desk", None)?;
    let meet = || -> TestResult {
        let request = sync_request(&phone, &desk.version_vector()?, "Pixel", "", "")?;
        let (answer, _) = answer_sync(&desk, &request, "Desk")?.ok_or("an answer")?;
        apply_answer(&phone, &answer)?;
        Ok(())
    };
    meet()?;
    let code = backup::set_up(&phone, "correct horse", CHEAP, &mut system_random)?;
    meet()?;
    assert_eq!(desk.recovery_code()?, Some(code.clone()));

    let dir = tempfile::tempdir()?;
    let sheet = dir.path().join("sheet.pdf");
    let saved = sheet.clone();
    let mut app = App::new(desk, "en");
    app.set_backup_dialogs(
        Box::new(move |_, _| Some(saved.clone())),
        Box::new(|_| None),
    );
    app.select(Selection::Profile);
    let mut harness = window(app);
    // Set up already: no passphrase asked, the sheet offered.
    assert!(!shows(&harness, "Set up Backups"));
    assert!(shows(&harness, "Recovery Sheet"));
    assert!(!shows(&harness, "Recovery keys of your other Devices"));
    click(&mut harness, "Save the Recovery Sheet…");
    let pdf = std::fs::read_to_string(&sheet)?;
    let first: Vec<&str> = code.split('-').take(7).collect();
    assert!(pdf.contains(&format!("({}) Tj", first.join("-"))));
    assert!(pdf.contains("(Profile: Ada) Tj"));
    Ok(())
}

/// Speakers that play nothing and keep every speed they were given.
struct Speakers(Rc<RefCell<Vec<f64>>>);

impl Player for Speakers {
    fn play(&mut self, _: Vec<u8>) -> bool {
        false
    }

    fn stop(&mut self) {}

    fn playing(&self) -> bool {
        false
    }

    fn set_speed(&mut self, speed: f64) {
        self.0.borrow_mut().push(speed);
    }
}

/// A Backup of a Device with a Note in an Area and a playback speed
/// chosen, and its recovery key.
fn old_backup(
    dir: &std::path::Path,
) -> Result<(std::path::PathBuf, String), Box<dyn std::error::Error>> {
    let old = store(7)?;
    let profile = oldbrainfix_core::profile::OwnProfile::new(&old);
    profile.apply_defaults("Desk", None)?;
    profile.rename("Ada")?;
    let area = Places::new(&old).add_area("Doctor")?;
    Notes::new(&old).capture("Pills at eight", Some(&area.id), None)?;
    DeviceSettings::new(&old).set_playback_speed(1.5)?;
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
    app.set_profile_defaults(Box::new(|store| {
        oldbrainfix_core::profile::OwnProfile::new(store).apply_defaults("Desk", None)
    }));
    app.offer_restore();
    assert!(app.welcoming());
    Ok(window(app))
}

#[test]
fn a_backup_of_a_later_format_says_an_update_is_needed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let later = dir.path().join(format!("later.{EXTENSION}"));
    std::fs::write(&later, b"{\"format\":2,\"type\":\"oldbrainfix-backup\"}\n")?;
    let mut harness = first_start(later)?;
    click(&mut harness, "Restore from a Backup");
    assert!(!shows(&harness, "Backup of "));
    assert_eq!(
        harness.state().notice(),
        Some("The Backup was not restored.")
    );
    harness.get_by_label("The Backup was not restored.").click();
    harness.run();
    assert!(shows(
        &harness,
        "This Backup was made by a newer Old Brain Fix."
    ));
    assert!(shows(
        &harness,
        "Update Old Brain Fix on this Device, then open it again."
    ));
    Ok(())
}

#[test]
fn the_first_start_restores_a_backup_with_the_passphrase() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, _) = old_backup(dir.path())?;
    let mut harness = first_start(path)?;
    let speeds = Rc::new(RefCell::new(Vec::new()));
    harness
        .state_mut()
        .set_player(Box::new(Speakers(speeds.clone())));
    assert_eq!(*speeds.borrow(), vec![1.0]);
    // No Profile of its own before the first start is answered.
    assert!(own_name(&harness)?.is_none());
    assert!(shows(&harness, "Welcome to Old Brain Fix"));
    click(&mut harness, "Restore from a Backup");
    assert!(shows(&harness, "Backup of "));

    type_into(&mut harness, "Passphrase", "wrong horse");
    click(&mut harness, "Restore");
    // Only that it was not restored; the click opens why and what to do.
    assert_eq!(
        harness.state().notice(),
        Some("The Backup was not restored.")
    );
    assert!(!shows(&harness, "does not open the Backup"));
    assert!(harness.state().welcoming());
    click(&mut harness, "The Backup was not restored.");
    assert!(shows(&harness, "Backup not restored"));
    assert!(shows(
        &harness,
        "This passphrase or recovery key does not open the Backup. Type the passphrase again, or use the recovery key on the Recovery Sheet."
    ));
    harness
        .get_all_by_label("Close")
        .last()
        .ok_or("Close")?
        .click_accesskit();
    harness.step();
    harness.step();

    type_over(&mut harness, Role::PasswordInput, 0, "correct horse");
    click(&mut harness, "Restore");
    assert!(!harness.state().welcoming());
    // Only that it is restored; the click shows how much and what to set
    // again.
    assert_eq!(harness.state().notice(), Some("Backup restored."));
    assert!(!shows(&harness, "Restored: "));
    click(&mut harness, "Backup restored.");
    assert!(shows(&harness, "Restored: "));
    harness
        .get_all_by_label("Close")
        .last()
        .ok_or("Close")?
        .click_accesskit();
    harness.step();
    harness.step();
    let store = harness.state().store();
    let texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    assert_eq!(texts, vec!["Pills at eight"]);
    // The playback speed came with it, for what plays from now on.
    assert_eq!(DeviceSettings::new(store).playback_speed()?, 1.5);
    assert_eq!(speeds.borrow().last(), Some(&1.5));
    // The Profile came back with it, and the Profile page shows it.
    harness.state_mut().select(Selection::Profile);
    harness.step();
    harness.step();
    assert!(harness.query_all_by_value("Ada").next().is_some());
    assert!(harness.query_all_by_value("Desk").next().is_none());
    harness.state_mut().select(Selection::All);
    harness.step();
    let own = oldbrainfix_core::profile::OwnProfile::new(harness.state().store()).get()?;
    assert_eq!(own.map(|p| p.name).as_deref(), Some("Ada"));
    // Not offered again.
    harness.state_mut().offer_restore();
    assert!(!harness.state().welcoming());
    Ok(())
}

#[test]
fn the_first_start_restores_with_the_recovery_key_or_starts_new() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, code) = old_backup(dir.path())?;
    let mut harness = first_start(path.clone())?;
    click(&mut harness, "Restore from a Backup");
    type_into(&mut harness, "Recovery key", "nonsense");
    click(&mut harness, "Restore");
    assert!(shows(&harness, "That is not a recovery key."));
    type_over(&mut harness, Role::TextInput, 0, &code.to_lowercase());
    click(&mut harness, "Restore");
    assert!(!harness.state().welcoming());
    assert_eq!(Places::new(harness.state().store()).areas()?.len(), 1);

    // Cancel goes back to the three choices, nothing answered.
    let mut harness = first_start(path)?;
    click(&mut harness, "Restore from a Backup");
    assert!(shows(&harness, "Backup of "));
    click(&mut harness, "Cancel");
    assert!(shows(&harness, "Add a Device of mine"));
    assert!(harness.state().welcoming());
    assert!(own_name(&harness)?.is_none());

    // New: the view goes and stays gone; only now is the Profile made.
    click(&mut harness, "New to Old Brain Fix");
    assert!(!harness.state().welcoming());
    assert_eq!(own_name(&harness)?.as_deref(), Some("Desk"));
    harness.state_mut().offer_restore();
    assert!(!harness.state().welcoming());
    assert!(Notes::new(harness.state().store()).all()?.is_empty());
    Ok(())
}
