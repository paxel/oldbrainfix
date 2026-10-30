//! Tables in formatted text in the window (#128, ADR 0033): a table reads
//! as a grid without its pipes, a click on it opens it full screen, back
//! leaves it; a cell is edited on one line, Enter writes it, Tab goes on,
//! Esc leaves it as it was; a row's or column's heading opens its menu;
//! the empty row adds one; the format bar puts a new table in; a cell's
//! status mark is changed in place; a Section's table opens to read only.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TABLE: &str = "| Item | Qty |\n| --- | --- |\n| milk | 2 |";

/// A store on a clock that moves a second per read, with its own
/// Profile.
fn device() -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([3; 32])))?;
    store.set_local_setting("author", "Desk")?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(store)
}

/// A store with the Note `text` in `format` filed in "Work", and its id.
fn with_note(text: &str, format: &str) -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device()?;
    let work = Places::new(&store).add_area("Work")?;
    let note = Notes::new(&store).capture_as(text, format, Some(&work.id), None)?;
    Ok((Arc::new(store), note.id))
}

/// The window over `store` in English with the Note `note` open in its
/// editor alone: it is filed, the Unsorted list shows no card of it.
fn editor(store: &Arc<Store>, note: &str) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Unsorted);
    app.select_note(note);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
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
    harness.query_all_by_label(label).next().is_some()
}

fn buttons(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

/// Clicks the last button `label` by the screen reader's click.
fn press(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Clicks the last button `label` with the pointer, as a cell takes it.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click();
    harness.run();
    Ok(())
}

fn typed(harness: &mut Harness<'_, App>, text: &str) {
    harness.event(egui::Event::Text(text.into()));
    harness.run();
}

fn key(harness: &mut Harness<'_, App>, key: Key) {
    harness.key_press(key);
    harness.run();
}

/// The text of `note` as it is stored, and its format.
fn stored(store: &Store, note: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let note = Notes::new(store).by_id(note)?.ok_or("the Note")?;
    Ok((note.text, note.text_format))
}

#[test]
fn a_table_reads_as_a_grid_without_its_pipes() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let harness = editor(&store, &note);
    for words in ["Item", "Qty", "milk", "2"] {
        assert!(shows(&harness, words), "{words}");
    }
    assert!(!shows(&harness, "| milk | 2 |"));
    assert!(!shows(&harness, "| --- | --- |"));
    assert_eq!(buttons(&harness, "Open the table"), 1);
    Ok(())
}

#[test]
fn a_click_opens_the_table_full_screen_and_back_leaves_it() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    assert_eq!(harness.state().shown_table(), Some(TABLE));
    // The rows' numbers and the columns' letters, where it may change.
    assert_eq!(buttons(&harness, "Row 1"), 1);
    assert_eq!(buttons(&harness, "Column B"), 1);
    key(&mut harness, Key::Escape);
    assert_eq!(harness.state().shown_table(), None);
    press(&mut harness, "Open the table")?;
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().shown_table(), None);
    Ok(())
}

#[test]
fn a_cell_is_written_on_enter_and_left_as_it_was_on_esc() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    click(&mut harness, "milk")?;
    // The cursor at the end of its words.
    typed(&mut harness, " | eggs");
    key(&mut harness, Key::Enter);
    let written = "| Item | Qty |\n| --- | --- |\n| milk \\| eggs | 2 |";
    assert_eq!(stored(&store, &note)?, (written.into(), MARKDOWN.into()));
    click(&mut harness, "2")?;
    typed(&mut harness, "0");
    key(&mut harness, Key::Escape);
    assert_eq!(stored(&store, &note)?.0, written);
    // Esc in a cell leaves the view open.
    assert!(harness.state().shown_table().is_some());
    Ok(())
}

#[test]
fn tab_writes_the_cell_and_goes_on_to_the_next() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    click(&mut harness, "milk")?;
    typed(&mut harness, "s");
    key(&mut harness, Key::Tab);
    typed(&mut harness, "0");
    key(&mut harness, Key::Enter);
    assert_eq!(
        stored(&store, &note)?.0,
        "| Item | Qty |\n| --- | --- |\n| milks | 20 |"
    );
    Ok(())
}

#[test]
fn the_empty_row_adds_a_row_when_typed_into() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    // The "+" beside the empty row, drawn after its cells, which say the
    // same: it edits the row's first cell.
    press(&mut harness, "Add a row")?;
    typed(&mut harness, "eggs");
    key(&mut harness, Key::Enter);
    assert_eq!(
        stored(&store, &note)?.0,
        "| Item | Qty |\n| --- | --- |\n| milk | 2 |\n| eggs |  |"
    );
    Ok(())
}

#[test]
fn a_row_holding_words_is_deleted_after_asking() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    press(&mut harness, "Row 2")?;
    press(&mut harness, "Delete row")?;
    assert!(shows(&harness, "Delete this row?"));
    assert_eq!(stored(&store, &note)?.0, TABLE);
    press(&mut harness, "Delete")?;
    assert_eq!(stored(&store, &note)?.0, "| Item | Qty |\n| --- | --- |");
    assert!(harness.state().shown_table().is_some());
    Ok(())
}

#[test]
fn the_first_columns_menu_marks_it_as_headings() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    press(&mut harness, "Column A")?;
    harness
        .query_all_by_label("First column as headings")
        .last()
        .ok_or("the switch")?
        .click_accesskit();
    harness.run();
    assert_eq!(
        stored(&store, &note)?.0,
        format!("<!-- first column headings -->\n{TABLE}")
    );
    Ok(())
}

#[test]
fn the_format_bar_puts_a_new_table_in_and_edits_its_first_cell() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    // The Note's bar, the first; "Add to Note" has one too.
    harness
        .query_all_by_role_and_label(Role::Button, "Table")
        .next()
        .ok_or("Table")?
        .click_accesskit();
    harness.run();
    let new = "|  |  |\n| --- | --- |\n|  |  |";
    // A field never typed into has its cursor at the start: before it.
    assert_eq!(
        stored(&store, &note)?,
        (format!("{new}\n\nShop"), MARKDOWN.into())
    );
    assert!(harness.state().shown_table().is_some());
    typed(&mut harness, "Item");
    key(&mut harness, Key::Enter);
    assert_eq!(
        stored(&store, &note)?.0,
        "| Item |  |\n| --- | --- |\n|  |  |\n\nShop"
    );
    Ok(())
}

#[test]
fn a_cells_status_mark_is_changed_in_place() -> TestResult {
    let (store, note) = with_note("| ☐ milk | 2 |\n| --- | --- |", MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open")?;
    press(&mut harness, "✅ Check")?;
    assert_eq!(stored(&store, &note)?.0, "| ✅ milk | 2 |\n| --- | --- |");
    Ok(())
}

#[test]
fn a_sections_table_opens_to_read_only() -> TestResult {
    let (store, note) = with_note("Shop", MARKDOWN)?;
    Notes::new(&store).amend_as(&note, TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    assert_eq!(harness.state().shown_table(), Some(TABLE));
    assert_eq!(buttons(&harness, "Row 1"), 0);
    assert_eq!(buttons(&harness, "Add a row"), 0);
    Ok(())
}
