//! A later AXT file of the same sender in the window (obf-1147, ADR
//! 0056): its update of a record taken before is listed ticked with the
//! Note it changes and "Take it" rewrites that Note; one the user changed
//! since is greyed with "Kept your change"; the same file opened again
//! names its record under "Not taken"; and a take of what was taken
//! meanwhile says that nothing changed.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::App;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Made<T> = Result<T, Box<dyn std::error::Error>>;

fn store(seed: u8, name: &str) -> Made<Store> {
    let store = Store::in_memory(Box::new(Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1100.0))
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

fn shows(harness: &Harness<'_, App>, text: &str) -> bool {
    harness.query_all_by_label_contains(text).next().is_some()
        || harness
            .query_all_by(|n| n.value().is_some_and(|v| v.contains(text)))
            .next()
            .is_some()
}

fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

fn record<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    harness.get_by_role_and_label(Role::CheckBox, label)
}

fn drop_file(harness: &mut Harness<'_, App>, path: &Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

/// Ann's Note "Dentist" written into `dir` as `name`, and her Note's id.
fn write(ann: &Store, id: &str, dir: &Path, name: &str) -> Made<PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, ann.axt_write(&[id.to_string()])?)?;
    Ok(path)
}

/// Desk, Ann, Ann's Note Dentist, and its first file taken in the window
/// into Unsorted; the id of the Note Desk took.
struct Taken {
    desk: Arc<Store>,
    ann: Store,
    dentist: String,
    note: String,
    first: PathBuf,
    harness: Harness<'static, App>,
    _dir: tempfile::TempDir,
}

fn taken() -> Made<Taken> {
    let desk = Arc::new(store(4, "Desk")?);
    let ann = store(1, "Ann")?;
    let dentist = Notes::new(&ann).capture("Dentist", None, None)?.id;
    let dir = tempfile::tempdir()?;
    let first = write(&ann, &dentist, dir.path(), "first.axt")?;
    let mut harness = window(App::new(desk.clone(), "en"));
    drop_file(&mut harness, &first);
    last(&harness, "Take it").click_accesskit();
    harness.run();
    assert_eq!(harness.state().notice(), Some("1 Note taken"));
    let note = Notes::new(&desk).all()?.first().ok_or("a Note")?.id.clone();
    Ok(Taken {
        desk,
        ann,
        dentist,
        note,
        first,
        harness,
        _dir: dir,
    })
}

#[test]
fn a_later_file_updates_the_note_taken_from_it() -> TestResult {
    let mut taken = taken()?;
    Notes::new(&taken.ann).edit(&taken.dentist, "Dentist\nOn Friday")?;
    let dir = tempfile::tempdir()?;
    let later = write(&taken.ann, &taken.dentist, dir.path(), "later.axt")?;
    let harness = &mut taken.harness;
    drop_file(harness, &later);
    let tick = record(harness, "Dentist").accesskit_node();
    assert_eq!(tick.toggled(), Some(Toggled::True));
    assert!(!tick.is_disabled());
    assert!(shows(harness, "Updates “Dentist”"));
    assert!(!shows(harness, "Not taken"));
    last(harness, "Take it").click_accesskit();
    harness.run();
    assert_eq!(harness.state().notice(), Some("1 record taken"));
    let text = Notes::new(&taken.desk)
        .by_id(&taken.note)?
        .ok_or("Note")?
        .text;
    assert_eq!(text, "Dentist\nOn Friday");
    assert_eq!(Notes::new(&taken.desk).all()?.len(), 1);
    Ok(())
}

#[test]
fn a_note_changed_since_keeps_the_change_greyed() -> TestResult {
    let mut taken = taken()?;
    Notes::new(&taken.desk).edit(&taken.note, "Dentist\nMy own words")?;
    Notes::new(&taken.ann).edit(&taken.dentist, "Dentist\nOn Friday")?;
    let dir = tempfile::tempdir()?;
    let later = write(&taken.ann, &taken.dentist, dir.path(), "later.axt")?;
    let harness = &mut taken.harness;
    drop_file(harness, &later);
    let tick = record(harness, "Dentist").accesskit_node();
    assert!(tick.is_disabled());
    assert_ne!(tick.toggled(), Some(Toggled::True));
    assert!(shows(harness, "Kept your change"));
    assert!(last(harness, "Take it").accesskit_node().is_disabled());
    Ok(())
}

#[test]
fn the_same_file_again_names_its_record_under_not_taken() -> TestResult {
    let mut taken = taken()?;
    let first = taken.first.clone();
    let harness = &mut taken.harness;
    drop_file(harness, &first);
    assert!(shows(harness, "Not taken"));
    assert!(shows(harness, "has this already"));
    assert!(record(harness, "Dentist").accesskit_node().is_disabled());
    assert!(last(harness, "Take it").accesskit_node().is_disabled());
    assert_eq!(Notes::new(&taken.desk).all()?.len(), 1);
    Ok(())
}

#[test]
fn a_take_of_what_was_taken_meanwhile_changes_nothing() -> TestResult {
    let mut taken = taken()?;
    Notes::new(&taken.ann).edit(&taken.dentist, "Dentist\nOn Friday")?;
    let dir = tempfile::tempdir()?;
    let later = write(&taken.ann, &taken.dentist, dir.path(), "later.axt")?;
    let harness = &mut taken.harness;
    drop_file(harness, &later);
    assert!(shows(harness, "Updates “Dentist”"));
    // Taken elsewhere while the dialog shows.
    let file = taken
        .desk
        .axt_read(&std::fs::read(&later)?, &utc)?
        .map_err(|_| "no AXT file")?;
    let ids: Vec<String> = file.records.iter().map(|r| r.id.clone()).collect();
    taken.desk.axt_take(&file, &ids, None)?;
    last(harness, "Take it").click_accesskit();
    harness.run();
    assert_eq!(
        harness.state().notice(),
        Some("Nothing changed: these records were taken before.")
    );
    assert_eq!(Notes::new(&taken.desk).all()?.len(), 1);
    Ok(())
}
