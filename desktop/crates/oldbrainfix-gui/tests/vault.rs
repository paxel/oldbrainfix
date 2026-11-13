//! The vault driven headless (#267, ADR 0050): File ▸ "Export as
//! vault…" asks for a folder, writes every Note into a new folder in it
//! off the window's thread and says so at the top, the folder's name
//! behind the click; the first start's "Rebuild from a vault" brings a
//! vault back into the empty app and answers the first start, and a
//! folder holding no Note rebuilds nothing, says why and stays on the
//! choices.

use std::path::{Path, PathBuf};
use std::time::Duration;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::vault::{self, VaultNames};
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::App;
use oldbrainfix_gui::backup_view::restore_due;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(seed: u8) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
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

/// The window of `app`, its folder dialog answering `picked` when asked
/// with `title`.
fn window(
    mut app: App,
    title: &'static str,
    picked: PathBuf,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    app.set_vault_picker(Box::new(move |asked| {
        assert_eq!(asked, title);
        Some(picked.clone())
    }));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
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

/// A fresh computer on its first start, its defaults naming the Profile
/// "Desk", its folder dialog answering `picked`.
fn first_start(picked: PathBuf) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store(1)?, "en");
    app.set_profile_defaults(Box::new(|store| {
        OwnProfile::new(store).apply_defaults("Desk", None)
    }));
    app.offer_restore();
    window(app, "Choose the vault's folder", picked)
}

/// The folders of `dir`.
fn folders(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    out.retain(|p| p.is_dir());
    Ok(out)
}

fn texts(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    texts.sort();
    Ok(texts)
}

#[test]
fn export_as_vault_writes_every_note_into_a_new_folder_and_says_so() -> TestResult {
    let desk = store(1)?;
    let health = Places::new(&desk).add_area("Health")?;
    Notes::new(&desk).capture("Dentist on Tuesday", Some(&health.id), None)?;
    let dir = tempfile::tempdir()?;
    let mut harness = window(
        App::new(desk, "en"),
        "Choose where the vault goes",
        dir.path().to_path_buf(),
    )?;
    harness.get_by_label("File").click();
    harness.step();
    harness.get_by_label("Export as vault…").click();
    harness.step();
    step_until(&mut harness, |app| !app.vault_running())?;
    harness.step();
    harness.step();
    // Only that it finished; the click says where it went.
    assert_eq!(harness.state().notice(), Some("Vault written."));
    let [vault] = folders(dir.path())?.try_into().map_err(|_| "one vault")?;
    let name = vault
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    assert!(name.starts_with("oldbrainfix-vault-"), "{name}");
    let notes: Vec<_> = std::fs::read_dir(vault.join("Health"))?.collect::<std::io::Result<_>>()?;
    assert_eq!(notes.len(), 1);
    harness.get_by_label("Vault written.").click_accesskit();
    harness.step();
    harness.step();
    let said = format!(
        "1 Note written into {}, with 0 files beside them.",
        vault.display()
    );
    assert!(shows(&harness, &said));
    Ok(())
}

#[test]
fn the_first_start_rebuilds_a_vault_and_goes_on() -> TestResult {
    let old = store(2)?;
    let health = Places::new(&old).add_area("Health")?;
    Notes::new(&old).capture("Dentist on Tuesday", Some(&health.id), None)?;
    Notes::new(&old).capture("Call Max", None, None)?;
    let dir = tempfile::tempdir()?;
    let area = |a: &oldbrainfix_core::Area| a.name.clone();
    let topic = |t: &oldbrainfix_core::Topic| t.name.clone();
    let names = VaultNames {
        area: &area,
        topic: &topic,
        unsorted: "Unsorted",
        untitled: "Note",
    };
    let utc = |at: chrono::DateTime<chrono::Utc>| at.naive_utc();
    let exported = vault::export(&old, dir.path(), &names, &utc, &|_, _| {})?;
    let mut harness = first_start(exported.folder)?;
    assert!(shows(&harness, "Rebuild from a vault"));
    harness
        .get_by_label("Rebuild from a vault")
        .click_accesskit();
    harness.step();
    step_until(&mut harness, |app| !app.welcoming())?;
    harness.step();
    harness.step();
    assert_eq!(harness.state().notice(), Some("Rebuilt from the vault."));
    let desk = harness.state().store();
    assert_eq!(texts(desk)?, texts(&old)?);
    // Answered: the own Profile is made now, and the view does not come
    // again.
    let own = OwnProfile::new(desk).get()?.ok_or("a Profile")?;
    assert_eq!(own.name, "Desk");
    assert!(!restore_due(desk)?);
    Ok(())
}

#[test]
fn a_folder_without_notes_rebuilds_nothing_and_says_why() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("photo.png"), [0x89, b'P', b'N', b'G'])?;
    let mut harness = first_start(dir.path().to_path_buf())?;
    harness
        .get_by_label("Rebuild from a vault")
        .click_accesskit();
    harness.step();
    step_until(&mut harness, |app| !app.vault_running())?;
    harness.step();
    harness.step();
    assert_eq!(harness.state().notice(), Some("Nothing was rebuilt."));
    harness
        .get_by_label("Nothing was rebuilt.")
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(
        &harness,
        "The folder holds no Markdown or text file."
    ));
    let app = harness.state();
    assert!(app.welcoming());
    assert!(restore_due(app.store())?);
    assert!(OwnProfile::new(app.store()).get()?.is_none());
    assert!(shows(&harness, "Restore from a Backup"));
    assert!(shows(&harness, "Rebuild from a vault"));
    Ok(())
}
