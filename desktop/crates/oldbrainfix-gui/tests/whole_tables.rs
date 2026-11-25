//! A table's ⋮ at its top right corner (obf-1050), driven headless by
//! egui_kittest: on a card it copies the table and opens nothing, no
//! card and no table full screen; in the Note dialog it also moves the
//! table past the block above or below it, greyed at the ends, and
//! deletes it, asked first while a cell holds words, each a step the
//! dialog's Undo takes back; in the grid edited the same; full screen and
//! in the Markdown viewer it copies.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::md_view::{Asked, Viewer};
use oldbrainfix_gui::media::Clipboard;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TABLE: &str = "| Item | Qty |\n| --- | --- |\n| milk | 2 |";
const MORE: &str = "More for this table";

/// A store on a clock that moves a microsecond per read.
fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([5; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(Arc::new(store))
}

/// The Note `text` filed in "Work", and its id.
fn note(store: &Store, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let work = Places::new(store).add_area("Work")?;
    Ok(Notes::new(store)
        .capture_as(text, MARKDOWN, Some(&work.id), None)?
        .id)
}

/// What went to the fake clipboard: each text with its HTML.
type Copied = Rc<RefCell<Vec<(String, Option<String>)>>>;

struct FakeClipboard(Copied);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn put_text(&mut self, text: &str, html: Option<&str>) -> bool {
        self.0
            .borrow_mut()
            .push((text.to_string(), html.map(str::to_string)));
        true
    }
}

/// The window over `store` in English listing All Notes, `open` open in
/// its dialog, its clipboard a fake one, and what went to it.
fn window(store: &Arc<Store>, open: Option<&str>) -> (Harness<'static, App>, Copied) {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    if let Some(note) = open {
        app.select_note(note);
    }
    let copied = Copied::default();
    app.set_clipboard(Box::new(FakeClipboard(copied.clone())));
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
    (harness, copied)
}

/// Clicks the last button `label` with the pointer: the dialog's over
/// the card's, a menu's over the window.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click();
    harness.run();
    Ok(())
}

/// Clicks the last widget `label` by the screen reader's click.
fn press(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Whether the last entry `label` can be clicked.
fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .query_all_by_label(label)
        .last()
        .is_some_and(|n| !n.accesskit_node().is_disabled())
}

fn text_of(store: &Store, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(id)?.ok_or("the Note")?.text)
}

/// What the Note's field shows.
fn field(harness: &Harness<'_, App>) -> String {
    harness
        .query_all_by_label("Note text")
        .last()
        .and_then(|n| n.value())
        .unwrap_or_default()
}

#[test]
fn a_cards_table_menu_copies_it_and_opens_nothing() -> TestResult {
    let store = store()?;
    let id = note(&store, &format!("Shop\n\n{TABLE}"))?;
    let (mut harness, copied) = window(&store, None);
    click(&mut harness, MORE)?;
    // Only read on a card: Copy alone.
    assert!(shows(&harness, "Copy"));
    assert!(!shows(&harness, "Delete table"));
    click(&mut harness, "Copy")?;
    let copied = copied.borrow();
    let [(markdown, html)] = copied.as_slice() else {
        return Err(format!("{copied:?}").into());
    };
    assert_eq!(markdown, TABLE);
    assert!(html.as_deref().is_some_and(|h| h.starts_with("<table>")));
    // No table full screen, no Note dialog, the Note as it was.
    assert_eq!(harness.state().shown_table(), None);
    assert!(!shows(&harness, "Note text"));
    assert_eq!(text_of(&store, &id)?, format!("Shop\n\n{TABLE}"));
    Ok(())
}

#[test]
fn the_dialogs_table_moves_past_the_block_above_and_undo_takes_it_back() -> TestResult {
    let store = store()?;
    let text = format!("Shop\n\n{TABLE}\n\nend");
    let id = note(&store, &text)?;
    let (mut harness, _) = window(&store, Some(&id));
    click(&mut harness, MORE)?;
    assert!(enabled(&harness, "Move up"));
    assert!(enabled(&harness, "Move down"));
    click(&mut harness, "Move up")?;
    let moved = format!("{TABLE}\n\nShop\n\nend");
    assert_eq!(field(&harness), moved);
    // At the top now: Move up greyed.
    click(&mut harness, MORE)?;
    assert!(!enabled(&harness, "Move up"));
    click(&mut harness, MORE)?;
    press(&mut harness, "Undo")?;
    assert_eq!(field(&harness), text);
    assert_eq!(text_of(&store, &id)?, text);
    Ok(())
}

#[test]
fn deleting_a_table_holding_words_asks_and_undo_brings_it_back() -> TestResult {
    let store = store()?;
    let text = format!("Shop\n\n{TABLE}\n\nend");
    let id = note(&store, &text)?;
    let (mut harness, _) = window(&store, Some(&id));
    click(&mut harness, MORE)?;
    click(&mut harness, "Delete table")?;
    assert!(shows(&harness, "Delete the table?"));
    // Cancel keeps it.
    press(&mut harness, "Cancel")?;
    assert_eq!(field(&harness), text);
    click(&mut harness, MORE)?;
    click(&mut harness, "Delete table")?;
    click(&mut harness, "Delete")?;
    assert_eq!(field(&harness), "Shop\n\n\nend");
    press(&mut harness, "Undo")?;
    assert_eq!(field(&harness), text);
    assert_eq!(text_of(&store, &id)?, text);
    Ok(())
}

#[test]
fn an_empty_table_is_deleted_without_asking() -> TestResult {
    let store = store()?;
    let empty = "|  |  |\n| --- | --- |";
    let id = note(&store, &format!("Shop\n\n{empty}"))?;
    let (mut harness, _) = window(&store, Some(&id));
    click(&mut harness, MORE)?;
    click(&mut harness, "Delete table")?;
    assert!(!shows(&harness, "Delete the table?"));
    assert_eq!(field(&harness), "Shop\n");
    Ok(())
}

#[test]
fn the_grid_edited_has_the_same_menu_its_move_a_step_after_the_typing() -> TestResult {
    let store = store()?;
    let text = format!("{TABLE}\n\nBread");
    let id = note(&store, &text)?;
    let (mut harness, copied) = window(&store, Some(&id));
    // Into a cell: the grid shows, its ⋮ at its top right corner.
    press(&mut harness, "Open the table")?;
    assert!(shows(&harness, "Row 1"));
    // Typed, then moved within the second typing rests for.
    harness.event(egui::Event::Text(" list".into()));
    harness.step();
    harness
        .query_all_by_role_and_label(Role::Button, MORE)
        .last()
        .ok_or(MORE)?
        .click();
    harness.step();
    assert!(!enabled(&harness, "Move up"));
    harness
        .query_all_by_label("Move down")
        .last()
        .ok_or("Move down")?
        .click_accesskit();
    harness.step();
    harness.step();
    let typed = TABLE.replace("Item", "Item list");
    assert_eq!(field(&harness), format!("Bread\n\n{typed}"));
    click(&mut harness, MORE)?;
    click(&mut harness, "Copy")?;
    assert_eq!(
        copied.borrow().first().map(|c| c.0.clone()),
        Some(typed.clone())
    );
    // Undo takes back the move alone, the typing stays.
    press(&mut harness, "Undo")?;
    assert_eq!(field(&harness), format!("{typed}\n\nBread"));
    press(&mut harness, "Undo")?;
    assert_eq!(field(&harness), text);
    Ok(())
}

#[test]
fn the_table_full_screen_copies_from_its_menu() -> TestResult {
    let store = store()?;
    note(&store, TABLE)?;
    let (mut harness, copied) = window(&store, None);
    press(&mut harness, "Open the table")?;
    assert!(harness.state().shown_table().is_some());
    click(&mut harness, MORE)?;
    click(&mut harness, "Copy")?;
    assert_eq!(copied.borrow().first().map(|c| c.0.as_str()), Some(TABLE));
    Ok(())
}

#[test]
fn the_markdown_viewer_copies_a_table_from_its_menu() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("shop.md");
    std::fs::write(&file, format!("# Shop\n\n{TABLE}\n"))?;
    let shown = Viewer::open(file, 1, egui::vec2(900.0, 700.0), None);
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
    for _ in 0..300 {
        harness.step();
        if harness.state().0.loaded() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    harness.run_steps(4);
    harness.get_by_role_and_label(Role::Button, MORE).click();
    harness.run_steps(4);
    harness
        .query_all_by_label("Copy")
        .last()
        .ok_or("Copy")?
        .click();
    harness.run_steps(4);
    let copied = harness.state().1.iter().find_map(|a| match a {
        Asked::Copy(copy) => Some(copy.markdown.clone()),
        _ => None,
    });
    assert_eq!(copied.as_deref(), Some(TABLE));
    Ok(())
}
