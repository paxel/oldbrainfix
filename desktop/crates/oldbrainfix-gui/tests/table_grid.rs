//! A table of a formatted text edited as a grid in place of its lines in
//! the Note dialog and Add to Note (#257, ADR 0033): a pasted table shows
//! as a grid, a click on a cell edits it and every change writes the
//! table's Markdown at once; Tab, Shift+Tab, Enter and the arrow keys go
//! from cell to cell and out into the text around, and back in; the "+"
//! and the rows' and columns' menus add, delete, move and line up; a
//! column's border drags it wider, nothing written; "Show as lines" and
//! "Show as table" switch the table's view; the format bar works on a
//! cell's words in Advanced and is gone in Simple, while the grid's own
//! buttons stay; the field stays edited while the keyboard is in a cell,
//! and leaving it keeps the text.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Checklists, Notes, OwnProfile, PersonSettings, Places, SigningKey, Store};
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
/// editor alone.
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

fn buttons(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

/// Clicks the first button `label` by the screen reader's click.
fn press(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .next()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Clicks the cell of the grid whose label begins with `cell`, "Row 2,
/// column A", with the pointer.
fn click_cell(harness: &mut Harness<'_, App>, cell: &str) -> TestResult {
    harness
        .query_all_by_role(Role::Button)
        .find(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| l.starts_with(cell))
        })
        .ok_or(format!("the cell {cell:?}"))?
        .click();
    harness.run();
    Ok(())
}

/// Clicks a heading's menu entry `label`, after the heading `heading`.
fn menu(harness: &mut Harness<'_, App>, heading: &str, label: &str) -> TestResult {
    press(harness, heading)?;
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?} in the menu of {heading:?}"))?
        .click_accesskit();
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

/// What the Note's text field holds.
fn field(harness: &Harness<'_, App>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(harness
        .query_all_by_role_and_label(Role::MultilineTextInput, "Note text")
        .next()
        .ok_or("the Note's field")?
        .value()
        .unwrap_or_default())
}

/// The text of `note` as it is stored.
fn stored(store: &Store, note: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(note)?.ok_or("the Note")?.text)
}

/// The Note's table edited, its header row's first cell with the
/// keyboard: a click on the table as it reads.
fn edit_table(harness: &mut Harness<'_, App>) -> TestResult {
    press(harness, "Open the table")?;
    harness.run();
    Ok(())
}

#[test]
fn a_pasted_table_shows_as_a_grid_in_place_of_its_lines() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    harness
        .query_all_by_role_and_label(Role::MultilineTextInput, "Note text")
        .next()
        .ok_or("the Note's field")?
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::End);
    harness.run();
    key(&mut harness, Key::Enter);
    harness.event(egui::Event::Paste(TABLE.into()));
    harness.run();
    // A grid, its rows numbered and its columns lettered, its cells by
    // their words; no table full screen.
    assert_eq!(buttons(&harness, "Row 2"), 1);
    assert_eq!(buttons(&harness, "Column B"), 1);
    assert_eq!(buttons(&harness, "Row 2, column A: milk"), 1);
    assert_eq!(harness.state().shown_table(), None);
    // The Markdown stays the source, kept formatted.
    assert!(field(&harness)?.starts_with(&format!("Shop\n{TABLE}")));
    let kept = Notes::new(&store).by_id(&note)?.ok_or("the Note")?;
    assert_eq!(kept.text_format, MARKDOWN);
    Ok(())
}

#[test]
fn a_click_on_a_cell_edits_it_and_writes_the_table_as_typed() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    click_cell(&mut harness, "Row 2, column A")?;
    // The cursor at the end of its words; a pipe stays in the cell.
    typed(&mut harness, " | eggs");
    let written = "| Item | Qty |\n| --- | --- |\n| milk \\| eggs | 2 |";
    assert!(
        field(&harness)?.starts_with(written),
        "{:?}",
        field(&harness)?
    );
    // Still edited, nothing kept yet: the keyboard is in the cell.
    assert_eq!(stored(&store, &note)?, TABLE);
    assert_eq!(buttons(&harness, "Row 1"), 1);
    // A space typed at its end stays while typing.
    typed(&mut harness, " ");
    typed(&mut harness, "x");
    assert!(field(&harness)?.contains("| milk \\| eggs x | 2 |"));
    // Leaving the field keeps it, as for any text.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert_eq!(
        stored(&store, &note)?,
        "| Item | Qty |\n| --- | --- |\n| milk \\| eggs x | 2 |"
    );
    Ok(())
}

#[test]
fn keys_go_from_cell_to_cell_and_out_into_the_text_and_back() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    // The header row's first cell, its cursor at the end.
    key(&mut harness, Key::Tab);
    typed(&mut harness, "s");
    key(&mut harness, Key::Tab);
    typed(&mut harness, "y");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    typed(&mut harness, "!");
    key(&mut harness, Key::Enter);
    typed(&mut harness, "2");
    assert!(
        field(&harness)?.starts_with("| Item | Qtys! |\n| --- | --- |\n| milky | 22 |"),
        "{:?}",
        field(&harness)?
    );
    // Left at a cell's start goes to the one before, at its end; Right at
    // a cell's end to the next, at its start.
    key(&mut harness, Key::ArrowLeft);
    key(&mut harness, Key::ArrowLeft);
    key(&mut harness, Key::ArrowLeft);
    typed(&mut harness, "<");
    key(&mut harness, Key::ArrowRight);
    typed(&mut harness, ">");
    assert!(
        field(&harness)?.contains("| milky< | >22 |"),
        "{:?}",
        field(&harness)?
    );
    // Up to the cell above; Down past the last row out after the table,
    // a blank line between, and typed there it is no row.
    key(&mut harness, Key::ArrowUp);
    typed(&mut harness, "^");
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::ArrowDown);
    typed(&mut harness, "Note");
    let text = field(&harness)?;
    assert!(text.contains("| Item | Qtys!^ |"), "{text:?}");
    assert!(text.ends_with("| milky< | >22 |\n\nNote"), "{text:?}");
    // Up from the text steps into the grid at the last cell's end.
    key(&mut harness, Key::ArrowUp);
    key(&mut harness, Key::ArrowUp);
    typed(&mut harness, "3");
    assert!(
        field(&harness)?.contains("| >223 |"),
        "{:?}",
        field(&harness)?
    );
    // Shift+Tab from the first cell goes out before the table, here a
    // line put in for it; Down from there goes back in at its start.
    for _ in 0..4 {
        harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
        harness.run();
    }
    typed(&mut harness, "Top");
    assert!(
        field(&harness)?.starts_with("Top\n| Item |"),
        "{:?}",
        field(&harness)?
    );
    key(&mut harness, Key::ArrowDown);
    typed(&mut harness, "A");
    assert!(
        field(&harness)?.starts_with("Top\n| AItem |"),
        "{:?}",
        field(&harness)?
    );
    // Esc goes back into the field, the dialog open.
    key(&mut harness, Key::Escape);
    assert_eq!(buttons(&harness, "Row 1"), 1);
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    Ok(())
}

#[test]
fn the_plus_and_the_menus_add_delete_move_and_line_up() -> TestResult {
    let (store, note) = with_note(&format!("{TABLE}\n\nend"), MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    // "+" under the rows adds one and edits its first cell.
    press(&mut harness, "Add a row")?;
    typed(&mut harness, "eggs");
    assert!(field(&harness)?.starts_with(&format!("{TABLE}\n| eggs |  |")));
    // "+" after the columns adds one and edits its header cell.
    press(&mut harness, "Add a column")?;
    typed(&mut harness, "Shop");
    assert!(
        field(&harness)?.starts_with("| Item | Qty | Shop |\n| --- | --- | --- |"),
        "{:?}",
        field(&harness)?
    );
    // Moved: the row up, the column left.
    menu(&mut harness, "Row 3", "Move up")?;
    menu(&mut harness, "Column C", "Move left")?;
    assert!(
        field(&harness)?.starts_with(
            "| Item | Shop | Qty |\n| --- | --- | --- |\n| eggs |  |  |\n| milk |  | 2 |"
        ),
        "{:?}",
        field(&harness)?
    );
    // A column lines up as picked, the pick checked.
    menu(&mut harness, "Column C", "Align right")?;
    assert!(field(&harness)?.contains("| --- | --- | --: |"));
    press(&mut harness, "Column C")?;
    let right = harness
        .query_all_by_label("Align right")
        .last()
        .ok_or("Align right")?
        .accesskit_node()
        .toggled();
    assert_eq!(right, Some(Toggled::True));
    // Picked again, it changes nothing.
    harness
        .query_all_by_label("Align right")
        .last()
        .ok_or("Align right")?
        .click_accesskit();
    harness.run();
    assert!(field(&harness)?.contains("| --- | --- | --: |"));
    // A column with words goes after asking, and so does a row.
    menu(&mut harness, "Column B", "Delete column")?;
    assert_eq!(buttons(&harness, "Delete"), 1);
    press(&mut harness, "Delete")?;
    assert!(
        field(&harness)?.starts_with("| Item | Qty |\n| --- | --: |"),
        "{:?}",
        field(&harness)?
    );
    menu(&mut harness, "Row 2", "Delete row")?;
    assert_eq!(buttons(&harness, "Delete"), 1);
    press(&mut harness, "Delete")?;
    assert!(
        field(&harness)?.starts_with("| Item | Qty |\n| --- | --: |\n| milk | 2 |"),
        "{:?}",
        field(&harness)?
    );
    // The field stayed edited throughout, nothing kept yet.
    assert_eq!(stored(&store, &note)?, format!("{TABLE}\n\nend"));
    // Ctrl+Z in the field takes the delete back.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(
        field(&harness)?.contains("| eggs |"),
        "{:?}",
        field(&harness)?
    );
    Ok(())
}

#[test]
fn a_columns_border_drags_it_wider_and_writes_nothing() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    key(&mut harness, Key::Escape);
    eprintln!(
        "rows {} field {:?}",
        buttons(&harness, "Row 1"),
        field(&harness)
    );
    let before = field(&harness)?;
    let column = |harness: &Harness<'_, App>| {
        harness
            .query_all_by_role_and_label(Role::Button, "Column A")
            .next()
            .map(|n| n.rect())
    };
    let rect = column(&harness).ok_or("Column A")?;
    let border = egui::pos2(rect.right(), rect.bottom() + 12.0);
    harness.hover_at(border);
    harness.run();
    harness.drag_at(border);
    harness.run();
    harness.hover_at(border + egui::vec2(60.0, 0.0));
    harness.run();
    harness.drop_at(border + egui::vec2(60.0, 0.0));
    harness.run();
    let wider = column(&harness).ok_or("Column A")?;
    assert!(wider.width() > rect.width() + 40.0, "{rect:?} {wider:?}");
    assert_eq!(field(&harness)?, before);
    Ok(())
}

#[test]
fn show_as_lines_and_show_as_grid_switch_the_tables_view() -> TestResult {
    let (store, note) = with_note(&format!("Shop\n\n{TABLE}"), MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    let before = field(&harness)?;
    press(&mut harness, "Show as lines")?;
    assert_eq!(buttons(&harness, "Row 1"), 0);
    assert_eq!(buttons(&harness, "Show as table"), 1);
    assert_eq!(field(&harness)?, before);
    // Typed into its lines, as text.
    key(&mut harness, Key::ArrowLeft);
    key(&mut harness, Key::ArrowLeft);
    typed(&mut harness, "s");
    assert!(
        field(&harness)?.contains("| Item | Qtys |"),
        "{:?}",
        field(&harness)?
    );
    press(&mut harness, "Show as table")?;
    assert_eq!(buttons(&harness, "Row 1"), 1);
    assert_eq!(buttons(&harness, "Show as table"), 0);
    // Shown as lines again, the cursor leaving its lines brings the grid
    // back.
    press(&mut harness, "Show as lines")?;
    assert_eq!(buttons(&harness, "Row 1"), 0);
    key(&mut harness, Key::ArrowUp);
    key(&mut harness, Key::ArrowUp);
    assert_eq!(buttons(&harness, "Row 1"), 1);
    // The format bar's Table in a table shown as lines shows its grid.
    press(&mut harness, "Show as lines")?;
    assert_eq!(buttons(&harness, "Row 1"), 0);
    press(&mut harness, "Table")?;
    assert_eq!(buttons(&harness, "Row 1"), 1);
    assert_eq!(table_count(&field(&harness)?), 1);
    Ok(())
}

/// How many tables `text` holds, by their delimiter rows.
fn table_count(text: &str) -> usize {
    text.lines().filter(|l| l.starts_with("| ---")).count()
}

#[test]
fn the_format_bar_works_on_a_cells_words_in_advanced_and_is_gone_in_simple() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    press(&mut harness, "Bold")?;
    assert!(
        field(&harness)?.starts_with("| **Item** | Qty |"),
        "{:?}",
        field(&harness)?
    );
    // Ctrl+I too, the cell keeping the keyboard.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::I);
    harness.run();
    let italic = field(&harness)?;
    let header = italic.lines().next().unwrap_or_default().to_string();
    assert!(header.matches('*').count() > 4, "{italic:?}");
    // A line format does nothing there.
    press(&mut harness, "Large heading")?;
    assert_eq!(field(&harness)?, italic);
    typed(&mut harness, "!");
    assert!(
        field(&harness)?
            .lines()
            .next()
            .is_some_and(|l| l.contains('!'))
    );
    // In Simple no bar, the grid's buttons as they were.
    PersonSettings::new(&store).set_advanced(false)?;
    let mut simple = editor(&store, &note);
    edit_table(&mut simple)?;
    assert_eq!(buttons(&simple, "Bold"), 0);
    assert_eq!(buttons(&simple, "Add a row"), 1);
    assert_eq!(buttons(&simple, "Show as lines"), 1);
    assert_eq!(buttons(&simple, "Row 1"), 1);
    Ok(())
}

#[test]
fn the_text_around_still_edits_and_leaving_the_field_keeps_it() -> TestResult {
    let (store, note) = with_note(&format!("Shop\n\n{TABLE}\n\nend"), MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    // Out after the table, typed into the text there.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::End);
    harness.run();
    for _ in 0..4 {
        key(&mut harness, Key::Tab);
    }
    typed(&mut harness, "the ");
    assert!(
        field(&harness)?.ends_with("\n\nthe end"),
        "{:?}",
        field(&harness)?
    );
    // Into a cell, and back into the text after it.
    click_cell(&mut harness, "Row 2, column B")?;
    typed(&mut harness, "0");
    key(&mut harness, Key::Escape);
    typed(&mut harness, "very ");
    let text = field(&harness)?;
    assert!(text.contains("| milk | 20 |"), "{text:?}");
    assert!(text.ends_with("the very end"), "{text:?}");
    // Into another field, the Note's is left: kept, and read again.
    harness
        .get_by(|n| n.placeholder() == Some("What's new?"))
        .click();
    harness.run();
    assert_eq!(
        stored(&store, &note)?,
        "Shop\n\n| Item | Qty |\n| --- | --- |\n| milk | 20 |\n\nthe very end"
    );
    assert_eq!(buttons(&harness, "Row 1"), 0);
    assert_eq!(buttons(&harness, "Open the table"), 1);
    Ok(())
}

#[test]
fn the_format_bar_puts_a_new_table_in_and_edits_its_first_cell_in_its_grid() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Table")?;
    let new = "|  |  |\n| --- | --- |\n|  |  |";
    // A field never typed into has its cursor at the start: before it.
    assert_eq!(stored(&store, &note)?, format!("{new}\n\nShop"));
    assert_eq!(harness.state().shown_table(), None);
    typed(&mut harness, "Item");
    assert!(
        field(&harness)?.starts_with("| Item |  |\n| --- | --- |"),
        "{:?}",
        field(&harness)?
    );
    Ok(())
}

#[test]
fn a_sections_table_is_edited_in_its_grid_too() -> TestResult {
    let (store, note) = with_note("Shop", MARKDOWN)?;
    let notes = Notes::new(&store);
    let amend = notes.amend_as(&note, TABLE, MARKDOWN)?.id;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    assert_eq!(harness.state().shown_table(), None);
    click_cell(&mut harness, "Row 2, column A")?;
    typed(&mut harness, "s");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    let section = notes.section_by_id(&amend)?.ok_or("the Amend")?;
    assert_eq!(section.text, "| Item | Qty |\n| --- | --- |\n| milks | 2 |");
    Ok(())
}

#[test]
fn a_table_of_add_to_note_is_edited_in_its_grid() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    // The Amend field's bar, the second.
    harness
        .query_all_by_role_and_label(Role::Button, "Table")
        .nth(1)
        .ok_or("the Table of Add to Note")?
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().shown_table(), None);
    typed(&mut harness, "Item");
    key(&mut harness, Key::Tab);
    typed(&mut harness, "Qty");
    let added = harness
        .query_all_by_role(Role::MultilineTextInput)
        .filter_map(|n| n.value())
        .find(|v| v.starts_with("| Item"))
        .ok_or("the Amend field")?;
    assert!(
        added.starts_with("| Item | Qty |\n| --- | --- |"),
        "{added:?}"
    );
    // Nothing of it in the Note until it is added.
    assert_eq!(stored(&store, &note)?, "Shop");
    assert!(Notes::new(&store).sections(&note)?.is_empty());
    Ok(())
}

#[test]
fn a_kinds_description_stays_open_while_a_cell_has_the_keyboard() -> TestResult {
    let store = device()?;
    let work = Places::new(&store).add_area("Work")?;
    let notes = Notes::new(&store);
    let id = notes.capture("Packing\nPassport", Some(&work.id), None)?.id;
    Checklists::new(&store).make(&id)?;
    notes.edit_as(&id, &format!("Packing\n{TABLE}"), MARKDOWN)?;
    let store = Arc::new(store);
    let mut harness = editor(&store, &id);
    harness
        .query_all_by_label("Show the whole description")
        .next()
        .ok_or("the description folded")?
        .click_accesskit();
    harness.run();
    click_cell(&mut harness, "Row 2, column A")?;
    typed(&mut harness, "s");
    let description = harness
        .query_all_by_role_and_label(Role::MultilineTextInput, "Description")
        .next()
        .ok_or("the description's field")?
        .value()
        .unwrap_or_default();
    assert!(description.contains("| milks | 2 |"), "{description:?}");
    assert_eq!(buttons(&harness, "Row 1"), 1);
    Ok(())
}

#[test]
fn a_cells_typing_undoes_in_the_field_as_one_step() -> TestResult {
    let (store, note) = with_note(&format!("{TABLE}\n\nend"), MARKDOWN)?;
    let mut harness = editor(&store, &note);
    edit_table(&mut harness)?;
    typed(&mut harness, "s");
    typed(&mut harness, "!");
    assert!(field(&harness)?.starts_with("| Items! | Qty |"));
    // Back in the field, Ctrl+Z takes the cell's typing back.
    key(&mut harness, Key::Escape);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(
        field(&harness)?.starts_with(TABLE),
        "{:?}",
        field(&harness)?
    );
    Ok(())
}

#[test]
fn a_cell_edited_as_the_dialog_closes_is_left_with_its_note() -> TestResult {
    let (store, first) = with_note(TABLE, MARKDOWN)?;
    let other = "| Day | Km |\n| --- | --- |\n| Mon | 5 |";
    let home = Places::new(&store).add_area("Home")?;
    let second = Notes::new(&store)
        .capture_as(other, MARKDOWN, Some(&home.id), None)?
        .id;
    let mut harness = editor(&store, &first);
    edit_table(&mut harness)?;
    typed(&mut harness, "s");
    // Straight to the other Note's text, edited, the cell still with
    // the keyboard.
    harness.state_mut().edit_note(&second);
    harness.run();
    let editing = harness
        .query_all_by_role(Role::TextInput)
        .filter(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| l.starts_with("Row 1, column A"))
        })
        .count();
    assert_eq!(editing, 0);
    // Its field never takes the first Note's text back.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(
        field(&harness)?.starts_with(other),
        "{:?}",
        field(&harness)?
    );
    assert!(stored(&store, &first)?.starts_with("| Items |"));
    Ok(())
}
