//! The viewer of a Markdown file on the desktop (obf-1056), driven
//! headless by egui_kittest, which draws its window inside the main one:
//! a file handed over from the command line or picked with File ▸ Open
//! Markdown file… shows in a window of its own, drawn as it reads, its
//! front matter as properties; nothing goes into the store; a wiki link
//! opens the file it names there and back returns, a name not found is
//! dimmed; the view is read again as the file changes; the find bar
//! finds in it; Import as Note asks about the files it links to and
//! imports them linked to each other, its pictures as photos.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::md_view::{Asked, Viewer};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GARDEN: &str = "---\ntitle: Garden plan\ntags:\n  - plants\n---\n\
# Garden\n\n\
Plant [[Roses|the roses]] and ==water== them, not [[Lost]].[^1]\n\n\
- [ ] dig\n- [x] seed\n  1. rye\n\n\
> [!tip] Mind the frost\n> Cover them at night.\n\n\
| Bed | Plants |\n|---|---:|\n| north | 12 |\n\n\
![[rose.png]]\n\n![[gone.png]]\n\n\
[^1]: Lost is a file not written yet.\n";

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let store = Store::in_memory(Box::new(move || start), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(Arc::new(store))
}

fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(8, 6, image::Rgba([20, 120, 200, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

/// A folder of Obsidian's with Garden.md, Roses.md and a picture: the
/// folder and Garden.md's path.
fn vault() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".obsidian")).unwrap();
    std::fs::create_dir_all(root.join("plants")).unwrap();
    std::fs::write(root.join("Garden.md"), GARDEN).unwrap();
    std::fs::write(
        root.join("plants/Roses.md"),
        "# Roses\n\nBack to the [[Garden]].\n",
    )
    .unwrap();
    std::fs::write(root.join("rose.png"), png()).unwrap();
    let garden = root.join("Garden.md");
    (dir, garden)
}

/// The main window over `store`, the files put into `handed` opening as
/// if handed over from outside.
fn window(store: &Arc<Store>, handed: Arc<Mutex<Vec<PathBuf>>>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    app.set_handed_files(handed);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1100.0))
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

/// Steps the window until `done` says so, the threads reading meanwhile.
fn settle(harness: &mut Harness<'_, App>, done: impl Fn(&App) -> bool) {
    for _ in 0..300 {
        harness.step();
        if done(harness.state()) {
            harness.run_steps(4);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("never settled");
}

/// Whether the first viewer has read the file it shows.
fn loaded(app: &App) -> bool {
    app.markdown_viewers().first().is_some_and(|v| v.loaded())
}

/// Whether a label shows `text`.
fn shows(harness: &Harness<'_, App>, text: &str) -> bool {
    harness
        .query_all_by(|n| n.value().is_some_and(|v| v.contains(text)))
        .next()
        .is_some()
}

/// Opens `file` as the command line does and waits until it is read.
fn open(harness: &mut Harness<'_, App>, handed: &Arc<Mutex<Vec<PathBuf>>>, file: &Path) {
    handed.lock().unwrap().push(file.to_path_buf());
    settle(harness, loaded);
}

#[test]
fn a_file_handed_over_shows_as_it_reads_and_nothing_goes_into_the_store() -> TestResult {
    let store = store()?;
    let before = store.all_entries()?.len();
    let (_dir, garden) = vault();
    let handed = Arc::default();
    let mut harness = window(&store, Arc::clone(&handed));
    open(&mut harness, &handed, &garden);
    assert_eq!(harness.state().markdown_viewers().len(), 1);
    for text in [
        "Garden plan",
        "plants",
        "Garden",
        "Plant the roses and water them, not Lost.[1]",
        "dig",
        "rye",
        "Mind the frost",
        "Cover them at night.",
        "Lost is a file not written yet.",
        "Picture not found: gone.png",
    ] {
        assert!(shows(&harness, text), "{text:?} not shown");
    }
    // The table as the app draws one, its cells as labels.
    assert!(shows(&harness, "north"));
    assert!(!shows(&harness, "Picture not found: rose.png"));
    // Its wiki links, the one found and the one dimmed, are links.
    harness.get_by_role_and_label(Role::Link, "the roses");
    harness.get_by_role_and_label(Role::Link, "Lost");
    // The front matter's marks do not show as text.
    assert!(!shows(&harness, "title: Garden plan"));
    // Opening it again brings it forward, no second viewer.
    handed.lock().unwrap().push(garden.clone());
    harness.run_steps(4);
    assert_eq!(harness.state().markdown_viewers().len(), 1);
    assert_eq!(store.all_entries()?.len(), before);
    Ok(())
}

#[test]
fn a_wiki_link_opens_its_file_in_the_viewer_and_back_returns() -> TestResult {
    let store = store()?;
    let (_dir, garden) = vault();
    let handed = Arc::default();
    let mut harness = window(&store, Arc::clone(&handed));
    open(&mut harness, &handed, &garden);
    harness
        .get_by_role_and_label(Role::Link, "Lost")
        .click_accesskit();
    harness.run_steps(4);
    assert!(
        shows(&harness, "Mind the frost"),
        "a dimmed link opens nothing"
    );
    harness
        .get_by_role_and_label(Role::Link, "the roses")
        .click_accesskit();
    settle(&mut harness, |app| {
        app.markdown_viewers()
            .first()
            .is_some_and(|v| v.loaded() && v.file().ends_with("plants/Roses.md"))
    });
    assert!(shows(&harness, "Back to the Garden."));
    assert!(!shows(&harness, "Mind the frost"));
    // The viewer's own Back, drawn after the list's.
    harness
        .query_all_by_label("Back")
        .next_back()
        .unwrap()
        .click_accesskit();
    settle(&mut harness, |app| {
        app.markdown_viewers()
            .first()
            .is_some_and(|v| v.loaded() && v.file().ends_with("Garden.md"))
    });
    assert!(shows(&harness, "Mind the frost"));
    Ok(())
}

#[test]
fn the_view_is_read_again_once_the_file_changes() -> TestResult {
    let store = store()?;
    let (_dir, garden) = vault();
    let handed = Arc::default();
    let mut harness = window(&store, Arc::clone(&handed));
    open(&mut harness, &handed, &garden);
    std::fs::write(&garden, "# Garden\n\nAll new, and longer than before.\n")?;
    for _ in 0..200 {
        harness.step();
        if shows(&harness, "All new, and longer than before.") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(shows(&harness, "All new, and longer than before."));
    assert!(!shows(&harness, "Mind the frost"));
    Ok(())
}

#[test]
fn find_in_its_menu_opens_the_find_bar_counting_the_places() -> TestResult {
    let store = store()?;
    let (_dir, garden) = vault();
    let handed = Arc::default();
    let mut harness = window(&store, Arc::clone(&handed));
    open(&mut harness, &handed, &garden);
    harness.get_by_label("More for this file").click();
    harness.run();
    harness.get_by_label("Find").click();
    harness.run_steps(4);
    harness
        .get_by_role_and_label(Role::TextInput, "Find in this file")
        .type_text("garden");
    harness.run_steps(6);
    // Its title, its heading and nothing else holds the word.
    assert!(shows(&harness, "1 of 2"));
    Ok(())
}

#[test]
fn file_menu_opens_a_markdown_file_picked() -> TestResult {
    let store = store()?;
    let (_dir, garden) = vault();
    let handed = Arc::default();
    let mut harness = window(&store, Arc::clone(&handed));
    let picked = garden.clone();
    harness
        .state_mut()
        .set_markdown_picker(Box::new(move |_| Some(picked.clone())));
    harness.get_by_label("File").click();
    harness.run();
    // Under Open ▸ (obf-1014).
    harness.get_by_label("Open ⏵").click();
    harness.run();
    harness.get_by_label("Markdown file…").click();
    settle(&mut harness, loaded);
    assert!(shows(&harness, "Mind the frost"));
    Ok(())
}

#[test]
fn import_as_note_asks_about_the_linked_files_and_links_their_notes() -> TestResult {
    let store = store()?;
    let (_dir, garden) = vault();
    let handed = Arc::default();
    let mut harness = window(&store, Arc::clone(&handed));
    open(&mut harness, &handed, &garden);
    harness
        .query_all_by(|n| {
            n.role() == Role::Button && n.label().as_deref() == Some("Import as Note…")
        })
        .next()
        .unwrap()
        .click();
    harness.run_steps(4);
    assert!(shows(&harness, "Also import the file it links to?"));
    harness.get_by_label("Import them too").click();
    let made = |store: &Store| {
        Places::new(store)
            .areas()
            .ok()
            .and_then(|areas| areas.into_iter().find(|a| a.name == "From Garden"))
    };
    for _ in 0..300 {
        harness.step();
        if made(&store).is_some() && !harness.state().importing() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let area = made(&store).ok_or("no Area From Garden")?;
    let notes = Notes::new(&store).inside(&area.id)?;
    assert_eq!(notes.len(), 2);
    let garden = notes
        .iter()
        .find(|n| n.text.starts_with("# Garden"))
        .unwrap();
    let roses = notes
        .iter()
        .find(|n| n.text.starts_with("# Roses"))
        .unwrap();
    assert!(
        garden
            .text
            .contains(&format!("[the roses](oldbrainfix://note/{})", roses.id))
    );
    assert!(
        roses
            .text
            .contains(&format!("[Garden](oldbrainfix://note/{})", garden.id))
    );
    assert_eq!(garden.photos.len(), 1);
    assert!(garden.text.contains("![[gone.png]]"));
    assert!(!garden.text.contains("![[rose.png]]"));
    assert!(garden.place.is_some());
    Ok(())
}

/// Clicks "Import as Note…" in the viewer, takes the linked file too and
/// steps until `done` says so.
fn import_as_note(harness: &mut Harness<'_, App>, done: impl Fn(&Harness<'_, App>) -> bool) {
    harness
        .query_all_by(|n| {
            n.role() == Role::Button && n.label().as_deref() == Some("Import as Note…")
        })
        .next()
        .unwrap()
        .click();
    harness.run_steps(4);
    harness.get_by_label("Import them too").click();
    for _ in 0..300 {
        harness.step();
        if done(harness) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("never got there");
}

#[test]
fn importing_the_file_again_asks_to_fill_in_and_finds_nothing_new() -> TestResult {
    let store = store()?;
    let (_dir, garden) = vault();
    let handed = Arc::default();
    let mut harness = window(&store, Arc::clone(&handed));
    open(&mut harness, &handed, &garden);
    import_as_note(&mut harness, |h| {
        h.state().notice().is_some() && !h.state().importing()
    });
    assert!(!shows(&harness, "Fill in the Import of"));
    import_as_note(&mut harness, |h| {
        shows(h, "Fill in the Import of Mon, 3/2/2026 into From Garden?")
    });
    harness.get_by_label("Fill in").click_accesskit();
    for _ in 0..300 {
        harness.step();
        if !harness.state().importing() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    harness.run_steps(2);
    assert_eq!(harness.state().notice(), Some("Nothing new to import."));
    let areas = Places::new(&store).areas()?;
    assert_eq!(
        areas
            .iter()
            .filter(|a| a.name.starts_with("From Garden"))
            .count(),
        1
    );
    Ok(())
}

/// A viewer of `file` alone at the window's root, its window's keys its
/// own as in a viewport of its own, with what it asked of the main
/// window.
fn viewer(file: &Path) -> Harness<'static, (Viewer, Vec<Asked>)> {
    let shown = Viewer::open(file.to_path_buf(), 1, egui::vec2(900.0, 700.0), None);
    let t = L10n::new("en");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(
            move |ui, (viewer, asked): &mut (Viewer, Vec<Asked>)| {
                App::install(ui.ctx());
                let (_, wanted) = viewer.frame(ui, &t, false, false);
                asked.extend(wanted);
            },
            (shown, Vec::new()),
        );
    viewer_settle(&mut harness, |v| v.loaded());
    harness
}

fn viewer_settle(harness: &mut Harness<'_, (Viewer, Vec<Asked>)>, done: impl Fn(&Viewer) -> bool) {
    for _ in 0..300 {
        harness.step();
        if done(&harness.state().0) {
            harness.run_steps(4);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("never settled");
}

/// A long file: `lines` paragraphs before its heading Summary and after.
fn long(title: &str, lines: usize) -> String {
    let mut text = format!("# {title}\n\n[[Roses#Summary]] [[#Notes]] Footnote[^1]\n\n");
    for i in 0..lines {
        text.push_str(&format!(
            "Paragraph {i} of {title} with some words to read.\n\n"
        ));
    }
    text.push_str("## Summary\n\nThe end of it.\n\n## Notes\n\nAll told.\n\n");
    for i in 0..lines {
        text.push_str(&format!("Later {i} of {title}.\n\n"));
    }
    text.push_str("[^1]: Far down.\n");
    text
}

#[test]
fn its_keys_find_change_the_text_size_and_go_back_and_forward() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("Garden.md"), long("Garden", 80))?;
    std::fs::write(dir.path().join("Roses.md"), long("Roses", 80))?;
    let mut harness = viewer(&dir.path().join("Garden.md"));
    // Ctrl+F opens the find bar, Esc closes it.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run_steps(4);
    harness.get_by_role_and_label(Role::TextInput, "Find in this file");
    harness.key_press(Key::Escape);
    harness.run_steps(4);
    assert!(
        harness
            .query_by_role_and_label(Role::TextInput, "Find in this file")
            .is_none()
    );
    // Ctrl+Plus and Ctrl+Minus ask the main window for the text size.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Plus);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Minus);
    harness.run_steps(2);
    assert_eq!(
        harness.state().1,
        [Asked::TextStep(true), Asked::TextStep(false)]
    );
    // A link to a heading of another file opens it there, not at the
    // same heading of this one.
    harness
        .get_by_role_and_label(Role::Link, "Roses > Summary")
        .click_accesskit();
    viewer_settle(&mut harness, |v| {
        v.loaded() && v.file().ends_with("Roses.md")
    });
    harness.run_steps(4);
    assert!(harness.state().0.scrolled().offset > 1000.0);
    // Alt+Left goes back to Garden at its top, Alt+Right forward again.
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    viewer_settle(&mut harness, |v| {
        v.loaded() && v.file().ends_with("Garden.md")
    });
    assert!(harness.state().0.scrolled().offset < 1.0);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    viewer_settle(&mut harness, |v| {
        v.loaded() && v.file().ends_with("Roses.md")
    });
    // A link to a heading of this file and a footnote scroll to them.
    harness.key_press(Key::Home);
    harness.run_steps(4);
    assert!(harness.state().0.scrolled().offset < 1.0);
    harness
        .get_by_role_and_label(Role::Link, "Notes")
        .click_accesskit();
    harness.run_steps(6);
    assert!(harness.state().0.scrolled().offset > 1000.0);
    Ok(())
}

#[test]
fn next_in_the_find_bar_brings_a_far_place_into_view() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("Garden.md"), long("Garden", 80))?;
    let mut harness = viewer(&dir.path().join("Garden.md"));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run_steps(4);
    harness
        .get_by_role_and_label(Role::TextInput, "Find in this file")
        .type_text("All told");
    harness.run_steps(8);
    assert!(
        harness
            .query_all_by(|n| n.value().is_some_and(|v| v == "1 of 1"))
            .next()
            .is_some()
    );
    assert!(harness.state().0.scrolled().offset > 1000.0);
    Ok(())
}

#[test]
fn a_file_near_the_largest_draws_within_the_budget() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut text = String::new();
    while text.len() < 1_000_000 {
        text.push_str("## A heading\n\nA paragraph with **bold** and *italic* words, a [[Link]] and `code`.\n\n- an item\n- another\n\n");
    }
    std::fs::write(dir.path().join("Big.md"), text)?;
    let mut harness = viewer(&dir.path().join("Big.md"));
    let start = std::time::Instant::now();
    harness.step();
    let took = start.elapsed();
    println!("a frame of a 1 MB file took {took:?}");
    assert!(took < std::time::Duration::from_millis(500), "{took:?}");
    Ok(())
}

/// A file alone in a folder of its own, away from the vault it links
/// into: with no Flatpak (obf-1151) it asks for no folder; its links and
/// pictures are found where it lies, or not at all.
#[test]
fn a_file_away_from_its_links_asks_for_no_folder() -> TestResult {
    let (_dir, garden) = vault();
    let elsewhere = tempfile::tempdir()?;
    let alone = elsewhere.path().join("Garden.md");
    std::fs::copy(&garden, &alone)?;
    let mut harness = viewer(&alone);
    harness.run_steps(2);
    let says = |text: &str| {
        harness
            .query_all_by(|n| {
                n.value().is_some_and(|v| v.contains(text))
                    || n.label().is_some_and(|l| l.contains(text))
            })
            .next()
            .is_some()
    };
    assert!(says("Picture not found: rose.png"));
    assert!(!says("Open the folder"));
    assert!(harness.state().1.is_empty());
    Ok(())
}
