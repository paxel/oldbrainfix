//! Sorting a table by a column (obf-1052), driven headless by
//! egui_kittest: where a table is only read, a click on a cell of its
//! header row shows its rows sorted by that column, ascending, then
//! descending, then as written, an arrow's direction read out, the
//! Note's text untouched and the order forgotten as the view goes; a
//! card's first 10 rows are the first of that order; the rows under the
//! header still open the table full screen. Full screen where the table
//! may change, the column's letter shows it sorted, until a row or column
//! is put in, moved or deleted; on the Note dialog's grid it sorts the
//! rows in the text, a step of Undo, none while they already stand so.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

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
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TABLE: &str =
    "| Item | Price |\n| --- | --- |\n| pears | 12,50 € |\n| apples | 3 € |\n| figs | 100 € |";
const PRICE: &str = "Price: Sort by this column";
const ITEM: &str = "Item: Sort by this column";

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

/// The window over `store` in English listing All Notes, `open` open in
/// its dialog.
fn window(store: &Arc<Store>, open: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    if let Some(note) = open {
        app.select_note(note);
    }
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

/// Clicks the last button `label` with the pointer.
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

fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    !harness
        .get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .is_disabled()
}

fn text_of(store: &Store, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(id)?.ok_or("the Note")?.text)
}

/// What the Note's field shows, without the blank lines the grid keeps
/// under its table.
fn field(harness: &Harness<'_, App>) -> String {
    harness
        .query_all_by_label("Note text")
        .last()
        .and_then(|n| n.value())
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

/// The words of `words` drawn, top to bottom, each by the first label
/// reading it.
fn rows(harness: &Harness<'_, App>, words: &[&str]) -> Vec<String> {
    let mut found: Vec<(f32, String)> = words
        .iter()
        .filter_map(|w| {
            let node = harness.query_all_by_value(w).next()?;
            Some((node.rect().top(), (*w).to_string()))
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, w)| w).collect()
}

/// What a screen reader hears of the heading `label`'s sort.
fn sorted(harness: &Harness<'_, App>, label: &str) -> Option<String> {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .and_then(|n| n.value())
}

const FRUIT: [&str; 3] = ["pears", "apples", "figs"];

/// The arrows drawn inside the heading `label`: "up" and "down".
fn arrows(harness: &Harness<'_, App>, label: &str) -> Vec<&'static str> {
    fn each(shape: &egui::Shape, inside: egui::Rect, out: &mut Vec<&'static str>) {
        match shape {
            egui::Shape::Text(text) if inside.contains_rect(text.visual_bounding_rect()) => {
                match text.galley.text() {
                    "\u{e0a0}" => out.push("up"),
                    "\u{e097}" => out.push("down"),
                    _ => {}
                }
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| each(s, inside, out)),
            _ => {}
        }
    }
    let Some(head) = harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
    else {
        return Vec::new();
    };
    let inside = head.rect().expand(1.0);
    let mut drawn = Vec::new();
    for clipped in &harness.output().shapes {
        each(&clipped.shape, inside, &mut drawn);
    }
    drawn
}

#[test]
fn a_click_on_a_cards_heading_sorts_the_rows_shown_up_down_and_back() -> TestResult {
    let store = store()?;
    let id = note(&store, TABLE)?;
    let mut harness = window(&store, None);
    assert_eq!(rows(&harness, &FRUIT), ["pears", "apples", "figs"]);
    assert_eq!(sorted(&harness, PRICE), None);
    assert!(arrows(&harness, PRICE).is_empty());
    // The pointer's click, over the card's and the table's.
    click(&mut harness, PRICE)?;
    assert_eq!(rows(&harness, &FRUIT), ["apples", "pears", "figs"]);
    assert_eq!(sorted(&harness, PRICE).as_deref(), Some("Sorted ascending"));
    assert_eq!(arrows(&harness, PRICE), ["up"]);
    assert_eq!(sorted(&harness, ITEM), None);
    assert!(arrows(&harness, ITEM).is_empty());
    click(&mut harness, PRICE)?;
    assert_eq!(rows(&harness, &FRUIT), ["figs", "pears", "apples"]);
    assert_eq!(
        sorted(&harness, PRICE).as_deref(),
        Some("Sorted descending")
    );
    assert_eq!(arrows(&harness, PRICE), ["down"]);
    click(&mut harness, PRICE)?;
    assert_eq!(rows(&harness, &FRUIT), ["pears", "apples", "figs"]);
    assert_eq!(sorted(&harness, PRICE), None);
    assert!(arrows(&harness, PRICE).is_empty());
    // Another column sorts by its words.
    click(&mut harness, ITEM)?;
    assert_eq!(rows(&harness, &FRUIT), ["apples", "figs", "pears"]);
    // In the view only: nothing opened, the text as written.
    assert_eq!(harness.state().shown_table(), None);
    assert!(!shows(&harness, "Note text"));
    assert_eq!(text_of(&store, &id)?, TABLE);
    Ok(())
}

#[test]
fn a_row_under_the_header_still_opens_the_table_full_screen() -> TestResult {
    let store = store()?;
    note(&store, TABLE)?;
    let mut harness = window(&store, None);
    click(&mut harness, PRICE)?;
    harness
        .query_all_by_value("apples")
        .next()
        .ok_or("apples")?
        .click();
    harness.run();
    assert_eq!(harness.state().shown_table(), Some(TABLE));
    Ok(())
}

#[test]
fn the_sort_is_forgotten_when_the_list_goes() -> TestResult {
    let store = store()?;
    note(&store, TABLE)?;
    let mut harness = window(&store, None);
    click(&mut harness, PRICE)?;
    assert_eq!(rows(&harness, &FRUIT), ["apples", "pears", "figs"]);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness.state_mut().select(Selection::All);
    harness.run();
    assert_eq!(rows(&harness, &FRUIT), ["pears", "apples", "figs"]);
    assert_eq!(sorted(&harness, PRICE), None);
    Ok(())
}

/// A table of `rows` rows under its header, "row 1" and so on, each
/// reading three times its number.
fn long_table(rows: usize) -> String {
    let mut text = String::from("| Day | Reading |\n| --- | --- |");
    for n in 1..=rows {
        text.push_str(&format!("\n| row {n} | {} |", n * 3));
    }
    text
}

#[test]
fn a_cards_ten_rows_are_the_first_of_the_order() -> TestResult {
    let store = store()?;
    note(&store, &long_table(30))?;
    let mut harness = window(&store, None);
    assert!(harness.query_all_by_value("row 10").next().is_some());
    assert!(harness.query_all_by_value("row 30").next().is_none());
    let reading = "Reading: Sort by this column";
    click(&mut harness, reading)?;
    click(&mut harness, reading)?;
    // Descending: rows 30 down to 21, none of the rest laid out.
    assert!(harness.query_all_by_value("row 30").next().is_some());
    assert!(harness.query_all_by_value("row 21").next().is_some());
    assert!(harness.query_all_by_value("row 20").next().is_none());
    assert!(harness.query_all_by_value("row 1").next().is_none());
    assert_eq!(rows(&harness, &["row 21", "row 30"]), ["row 30", "row 21"]);
    assert!(shows(&harness, "Show all 30"));
    Ok(())
}

#[test]
fn the_table_full_screen_only_read_sorts_by_a_click_on_its_heading() -> TestResult {
    let store = store()?;
    let id = note(&store, "Shop")?;
    let notes = Notes::new(&store);
    notes.amend_as(&id, TABLE, MARKDOWN)?;
    notes.trash(&id)?;
    let mut harness = window(&store, Some(&id));
    press(&mut harness, "Open the table")?;
    assert_eq!(harness.state().shown_table(), Some(TABLE));
    assert!(!shows(&harness, "Column B"));
    click(&mut harness, PRICE)?;
    assert_eq!(sorted(&harness, PRICE).as_deref(), Some("Sorted ascending"));
    // Its cells read under the dialog's: the last of each.
    let last = |harness: &Harness<'_, App>| {
        let mut found: Vec<(f32, String)> = FRUIT
            .iter()
            .filter_map(|w| {
                let node = harness.query_all_by_value(w).last()?;
                Some((node.rect().top(), (*w).to_string()))
            })
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found.into_iter().map(|(_, w)| w).collect::<Vec<_>>()
    };
    assert_eq!(last(&harness), ["apples", "pears", "figs"]);
    click(&mut harness, PRICE)?;
    assert_eq!(last(&harness), ["figs", "pears", "apples"]);
    assert_eq!(harness.state().shown_table(), Some(TABLE));
    Ok(())
}

#[test]
fn the_table_full_screen_edited_shows_it_sorted_by_the_columns_letter() -> TestResult {
    let store = store()?;
    let id = note(&store, TABLE)?;
    let mut harness = window(&store, None);
    press(&mut harness, "Open the table")?;
    assert!(shows(&harness, "Column B"));
    // A click on the header row edits it there: no sorting button but
    // the card's under it.
    let heads = harness.query_all_by_role_and_label(Role::Button, PRICE);
    assert_eq!(heads.count(), 1);
    press(&mut harness, "Column B")?;
    assert!(shows(&harness, "Show sorted ascending"));
    assert!(shows(&harness, "Show sorted descending"));
    assert!(!shows(&harness, "Show as written"));
    press(&mut harness, "Show sorted descending")?;
    let cells = |harness: &Harness<'_, App>| {
        let mut found: Vec<(f32, String)> = FRUIT
            .iter()
            .filter_map(|w| {
                let node = harness
                    .query_all_by_role_and_label(Role::Button, w)
                    .last()?;
                Some((node.rect().top(), (*w).to_string()))
            })
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found.into_iter().map(|(_, w)| w).collect::<Vec<_>>()
    };
    assert_eq!(cells(&harness), ["figs", "pears", "apples"]);
    assert_eq!(text_of(&store, &id)?, TABLE);
    press(&mut harness, "Column B")?;
    assert!(shows(&harness, "Show sorted ascending"));
    assert!(!shows(&harness, "Show sorted descending"));
    assert!(shows(&harness, "Show as written"));
    press(&mut harness, "Show as written")?;
    assert_eq!(cells(&harness), ["pears", "apples", "figs"]);
    // Sorted again, a row put in shows them as written.
    press(&mut harness, "Column B")?;
    press(&mut harness, "Show sorted ascending")?;
    assert_eq!(cells(&harness), ["apples", "pears", "figs"]);
    press(&mut harness, "Row 2")?;
    press(&mut harness, "Insert a row below")?;
    assert!(harness.state().shown_table().is_some());
    assert_eq!(cells(&harness), ["pears", "apples", "figs"]);
    Ok(())
}

#[test]
fn the_grids_column_sorts_the_rows_in_the_text_a_step_of_undo() -> TestResult {
    let store = store()?;
    let id = note(&store, TABLE)?;
    let mut harness = window(&store, Some(&id));
    press(&mut harness, "Open the table")?;
    assert!(shows(&harness, "Row 1"));
    harness
        .query_all_by_role_and_label(Role::Button, "Column B")
        .last()
        .ok_or("Column B")?
        .click();
    harness.step();
    harness
        .query_all_by_label("Sort rows by this column, descending")
        .last()
        .ok_or("Sort rows by this column, descending")?
        .click_accesskit();
    harness.step();
    harness.step();
    let down =
        "| Item | Price |\n| --- | --- |\n| figs | 100 € |\n| pears | 12,50 € |\n| apples | 3 € |";
    assert_eq!(field(&harness), down);
    // Sorted so again: no step, one Undo takes it back.
    harness
        .query_all_by_role_and_label(Role::Button, "Column B")
        .last()
        .ok_or("Column B")?
        .click();
    harness.step();
    harness
        .query_all_by_label("Sort rows by this column, descending")
        .last()
        .ok_or("Sort rows by this column, descending")?
        .click_accesskit();
    harness.step();
    harness.step();
    // The menu closed: its entry was taken.
    assert!(!shows(&harness, "Sort rows by this column, descending"));
    assert_eq!(field(&harness), down);
    press(&mut harness, "Undo")?;
    assert_eq!(field(&harness), TABLE);
    assert!(!enabled(&harness, "Undo"));
    Ok(())
}

#[test]
fn rows_already_sorted_are_no_step_of_undo() -> TestResult {
    let store = store()?;
    let up = "| Item | Price |\n| --- | --- |\n| apples | 3 € |\n| pears | 12,50 € |";
    let id = note(&store, up)?;
    let mut harness = window(&store, Some(&id));
    press(&mut harness, "Open the table")?;
    harness
        .query_all_by_role_and_label(Role::Button, "Column B")
        .last()
        .ok_or("Column B")?
        .click();
    harness.step();
    harness
        .query_all_by_label("Sort rows by this column")
        .last()
        .ok_or("Sort rows by this column")?
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(!shows(&harness, "Sort rows by this column"));
    assert_eq!(field(&harness), up);
    assert!(!enabled(&harness, "Undo"));
    Ok(())
}

#[test]
fn the_markdown_viewer_sorts_a_table_by_its_heading() -> TestResult {
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
    harness.get_by_role_and_label(Role::Button, PRICE).click();
    harness.run_steps(4);
    let mut found: Vec<(f32, &str)> = FRUIT
        .iter()
        .filter_map(|w| Some((harness.query_all_by_value(w).next()?.rect().top(), *w)))
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    let words: Vec<&str> = found.into_iter().map(|(_, w)| w).collect();
    assert_eq!(words, ["apples", "pears", "figs"]);
    assert!(harness.state().1.is_empty());
    Ok(())
}
