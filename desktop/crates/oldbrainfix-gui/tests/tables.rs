//! Tables in formatted text in the window (#128, ADR 0033): a table reads
//! as a grid without its pipes, a click on it on a card opens it full
//! screen, back leaves it; a cell is edited on one line, Enter writes it,
//! Tab goes on, Esc leaves it as it was; a row's or column's heading opens
//! its menu; the empty row adds one; a cell's box ticks in place, its
//! text as it was; a table of a Note in the trash opens to read only;
//! Copy puts the table on the clipboard as an HTML table beside its
//! Markdown (#324). The editor edits a table as a grid in place (#257,
//! tests/table_grid.rs). On a card a table keeps the text's size, wider
//! than the card it moves sideways, its first column staying while it
//! holds headings, and it shows its first 10 rows until "Show all"
//! unfolds the rest in place (obf-1048).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::markdown::table;
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::media::Clipboard;
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

/// The window over `store` in English showing every Note's card, none
/// open in the editor.
fn cards(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
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

/// [`cards`], its clipboard a fake one, and what went to it.
fn cards_copying(store: &Arc<Store>) -> (Harness<'static, App>, Copied) {
    let copied = Copied::default();
    let mut harness = cards(store);
    harness
        .state_mut()
        .set_clipboard(Box::new(FakeClipboard(copied.clone())));
    (harness, copied)
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
    let (store, _) = with_note(TABLE, MARKDOWN)?;
    let mut harness = cards(&store);
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
    let mut harness = cards(&store);
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
    let mut harness = cards(&store);
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
    let mut harness = cards(&store);
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
    let mut harness = cards(&store);
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
    let mut harness = cards(&store);
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
fn a_cells_box_ticks_in_place() -> TestResult {
    let (store, note) = with_note("| ☐ milk | 2 |\n| --- | --- |", MARKDOWN)?;
    let before = stored(&store, &note)?.0;
    let mut harness = cards(&store);
    press(&mut harness, "Open")?;
    press(&mut harness, "✅ Check")?;
    // A tick beside the text, which stays as it was.
    assert_eq!(stored(&store, &note)?.0, before);
    let shown = Notes::new(&store).by_id(&note)?.ok_or("the Note")?;
    assert_eq!(shown.shown_text(), "| ✅ milk | 2 |\n| --- | --- |");
    assert_eq!(buttons(&harness, "Checked"), 1);
    // In the table full screen too, and there it ticks back.
    press(&mut harness, "Open the table")?;
    assert!(harness.state().shown_table().is_some());
    press(&mut harness, "Checked")?;
    press(&mut harness, "☐ Open")?;
    let shown = Notes::new(&store).by_id(&note)?.ok_or("the Note")?;
    assert_eq!(shown.shown_text(), "| ☐ milk | 2 |\n| --- | --- |");
    assert_eq!(stored(&store, &note)?.0, before);
    Ok(())
}

#[test]
fn a_table_in_the_trash_opens_to_read_only() -> TestResult {
    let (store, note) = with_note("Shop", MARKDOWN)?;
    let notes = Notes::new(&store);
    notes.amend_as(&note, TABLE, MARKDOWN)?;
    notes.trash(&note)?;
    let mut harness = editor(&store, &note);
    press(&mut harness, "Open the table")?;
    assert_eq!(harness.state().shown_table(), Some(TABLE));
    assert_eq!(buttons(&harness, "Row 1"), 0);
    assert_eq!(buttons(&harness, "Add a row"), 0);
    Ok(())
}

#[test]
fn copy_puts_the_table_on_the_clipboard_as_html_beside_its_markdown() -> TestResult {
    // #324: each box in the state it shows, without its id; the HTML a
    // table with its header row in <thead>, each cell escaped and
    // formatted; nothing announced.
    let text = "Shop\n\n| Item | Qty |\n| :-- | --: |\n| ☐ **milk** | 2 <3 |";
    let (store, note) = with_note(text, MARKDOWN)?;
    let before = stored(&store, &note)?.0;
    let (mut harness, copied) = cards_copying(&store);
    press(&mut harness, "Open")?;
    press(&mut harness, "✅ Check")?;
    press(&mut harness, "Open the table")?;
    press(&mut harness, "Copy")?;
    let markdown = "| Item | Qty |\n| :-- | --: |\n| ✅ **milk** | 2 <3 |";
    let html = "<table><thead><tr><th align=\"left\">Item</th><th align=\"right\">Qty</th>\
                </tr></thead><tbody><tr><td align=\"left\">✅ <strong>milk</strong></td>\
                <td align=\"right\">2 &lt;3</td></tr></tbody></table>\n";
    assert_eq!(
        copied.borrow().as_slice(),
        [(markdown.to_string(), Some(html.to_string()))]
    );
    // The table stays open, the Note as it was.
    assert!(harness.state().shown_table().is_some());
    assert_eq!(stored(&store, &note)?.0, before);
    Ok(())
}

#[test]
fn copy_writes_the_cell_being_edited_first() -> TestResult {
    let (store, note) = with_note(TABLE, MARKDOWN)?;
    let (mut harness, copied) = cards_copying(&store);
    press(&mut harness, "Open the table")?;
    click(&mut harness, "2")?;
    typed(&mut harness, "0");
    click(&mut harness, "Copy")?;
    let written = "| Item | Qty |\n| --- | --- |\n| milk | 20 |";
    assert_eq!(stored(&store, &note)?.0, written);
    let copied = copied.borrow();
    let [(text, html)] = copied.as_slice() else {
        return Err(format!("{copied:?}").into());
    };
    assert_eq!(text, written);
    assert!(html.as_deref().is_some_and(|h| h.contains("<td>20</td>")));
    Ok(())
}

#[test]
fn a_table_that_is_only_read_copies_too() -> TestResult {
    let (store, note) = with_note("Shop", MARKDOWN)?;
    let notes = Notes::new(&store);
    notes.amend_as(&note, TABLE, MARKDOWN)?;
    notes.trash(&note)?;
    let copied = Copied::default();
    let mut harness = editor(&store, &note);
    harness
        .state_mut()
        .set_clipboard(Box::new(FakeClipboard(copied.clone())));
    press(&mut harness, "Open the table")?;
    press(&mut harness, "Copy")?;
    assert_eq!(copied.borrow().first().map(|c| c.0.as_str()), Some(TABLE));
    Ok(())
}

#[test]
fn without_the_systems_clipboard_copy_puts_the_markdown_through_the_window() -> TestResult {
    // #324: where HTML cannot go along, the Markdown alone.
    let (store, _) = with_note(TABLE, MARKDOWN)?;
    let mut harness = cards(&store);
    press(&mut harness, "Open the table")?;
    harness
        .query_all_by_role_and_label(Role::Button, "Copy")
        .last()
        .ok_or("Copy")?
        .click_accesskit();
    let mut copied = None;
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                copied = Some(text.clone());
            }
        }
    }
    harness.run();
    assert_eq!(copied.as_deref(), Some(TABLE));
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn a_copied_table_pasted_into_a_new_note_is_the_same_table() -> TestResult {
    // #324: its rows, its columns' alignment and its first column as
    // headings come back.
    let text = "<!-- first column headings -->\n| Meter | Jan |\n| :-- | --: |\n\
                | Water \\| m³ | **312** |";
    let (store, _) = with_note(text, MARKDOWN)?;
    let (mut harness, copied) = cards_copying(&store);
    press(&mut harness, "Open the table")?;
    press(&mut harness, "Copy")?;
    let markdown = copied.borrow().first().ok_or("nothing copied")?.0.clone();
    let pasted = Notes::new(&store).capture_as(&markdown, MARKDOWN, None, None)?;
    let shape = |text: &str| {
        table::all(text).first().map(|t| {
            let rows: Vec<Vec<String>> = t
                .rows
                .iter()
                .map(|r| r.cells.iter().map(|c| c.visible()).collect())
                .collect();
            (rows, t.aligns.clone(), t.heading_column)
        })
    };
    let before = shape(text).ok_or("the table")?;
    assert!(before.2);
    assert_eq!(shape(&pasted.text), Some(before));
    Ok(())
}

/// A table of `rows` rows under its header, "row 1" and so on.
fn long_table(rows: usize) -> String {
    let mut text = String::from("| Day | Reading |\n| --- | --- |");
    for n in 1..=rows {
        text.push_str(&format!("\n| row {n} | {} |", n * 3));
    }
    text
}

/// A table of eight columns, far wider than a card, its first column as
/// headings, under a line of text.
fn wide_table() -> String {
    let header: Vec<String> = (1..=8).map(|c| format!("Meter number {c}")).collect();
    let body: Vec<String> = (1..=8)
        .map(|c| format!("reading {c} of the year"))
        .collect();
    format!(
        "Before\n\n<!-- first column headings -->\n| {} |\n|{}\n| {} |",
        header.join(" | "),
        " --- |".repeat(8),
        body.join(" | ")
    )
}

/// Where the label reading `value`, a table's cell, lies.
fn cell(harness: &Harness<'_, App>, value: &str) -> Result<egui::Rect, String> {
    harness
        .query_all_by_value(value)
        .next()
        .map(|n| n.rect())
        .ok_or(format!("{value:?}"))
}

/// Whether the words `words` are drawn, each time wholly inside what
/// they are clipped to: what accesskit gives is where a cell lies, not
/// what of it shows.
fn drawn_whole(harness: &Harness<'_, App>, words: &str) -> bool {
    fn each(shape: &egui::Shape, clip: egui::Rect, words: &str, out: &mut Vec<bool>) {
        match shape {
            egui::Shape::Text(text) if text.galley.text() == words => {
                out.push(clip.expand(1.0).contains_rect(text.visual_bounding_rect()));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| each(s, clip, words, out)),
            _ => {}
        }
    }
    let mut drawn = Vec::new();
    for clipped in &harness.output().shapes {
        each(&clipped.shape, clipped.clip_rect, words, &mut drawn);
    }
    !drawn.is_empty() && drawn.iter().all(|&whole| whole)
}

#[test]
fn a_card_shows_ten_rows_of_a_long_table_until_show_all() -> TestResult {
    let (store, _) = with_note(&long_table(200), MARKDOWN)?;
    let mut harness = cards(&store);
    cell(&harness, "row 10")?;
    // The rows not shown are not laid out (the speed budget, obf-1069).
    assert!(cell(&harness, "row 11").is_err());
    assert_eq!(buttons(&harness, "Show all 200"), 1);
    // The pointer's click, over the card's own.
    click(&mut harness, "Show all 200")?;
    cell(&harness, "row 200")?;
    assert_eq!(buttons(&harness, "Show all 200"), 0);
    assert_eq!(harness.state().shown_table(), None);
    assert_eq!(buttons(&harness, "Open the table"), 1);
    Ok(())
}

#[test]
fn a_table_of_ten_rows_shows_them_all_without_show_all() -> TestResult {
    let (store, _) = with_note(&long_table(10), MARKDOWN)?;
    let harness = cards(&store);
    cell(&harness, "row 10")?;
    let more = harness.query_all_by(|n| n.label().is_some_and(|l| l.starts_with("Show all")));
    assert_eq!(more.count(), 0);
    Ok(())
}

#[test]
fn a_wide_table_keeps_the_texts_size_and_moves_sideways_by_the_wheel() -> TestResult {
    let (store, _) = with_note(&wide_table(), MARKDOWN)?;
    let mut harness = cards(&store);
    // As high as the line of text above it: not made smaller.
    let before = harness
        .query_all_by_value("Before")
        .next()
        .ok_or("the line before")?
        .rect();
    let body = cell(&harness, "reading 5 of the year")?;
    assert!(
        (body.height() - before.height()).abs() < 1.5,
        "{body:?} {before:?}"
    );
    // Its last column lies past the card's edge.
    let table = harness
        .query_all_by_role_and_label(Role::Button, "Open the table")
        .next()
        .ok_or("the table")?
        .rect();
    let last = cell(&harness, "reading 8 of the year")?;
    assert!(last.right() > table.right() + 50.0, "{last:?} {table:?}");
    assert!(!drawn_whole(&harness, "reading 8 of the year"));
    assert!(drawn_whole(&harness, "reading 1 of the year"));
    let first = cell(&harness, "reading 1 of the year")?;
    harness.hover_at(table.center());
    harness.run();
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-300.0, 0.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    // The wheel's step is spread over a few frames.
    harness.run_steps(30);
    let moved = cell(&harness, "reading 8 of the year")?;
    assert!(moved.left() < last.left() - 100.0, "{moved:?} {last:?}");
    // The first column holds headings: it stays.
    let stayed = cell(&harness, "reading 1 of the year")?;
    assert!(
        (stayed.left() - first.left()).abs() < 0.5,
        "{stayed:?} {first:?}"
    );
    // The list did not scroll.
    let still = harness
        .query_all_by_value("Before")
        .next()
        .ok_or("the line before")?
        .rect();
    assert!((still.top() - before.top()).abs() < 0.5);
    Ok(())
}

#[test]
fn a_long_cell_wraps_within_the_card() -> TestResult {
    let long = "Pay the gas bill before the end of the month and keep the receipt \
                in the blue folder in the hall, next to the one for the water, \
                then call the office to say it was paid, and ask about the meter";
    let text = format!("| Task | When |\n| --- | --- |\n| {long} | Friday |");
    let (store, _) = with_note(&text, MARKDOWN)?;
    let harness = cards(&store);
    let table = harness
        .query_all_by_role_and_label(Role::Button, "Open the table")
        .next()
        .ok_or("the table")?
        .rect();
    let words = cell(&harness, long)?;
    let short = cell(&harness, "Friday")?;
    assert!(words.right() <= table.right(), "{words:?} {table:?}");
    // On several lines, each at the text's size.
    assert!(words.height() > short.height() * 1.5, "{words:?} {short:?}");
    // It fits: no bar under it.
    assert!(cell(&harness, "Friday")?.right() <= table.right());
    Ok(())
}

#[test]
fn the_bar_under_a_wide_table_on_a_card_moves_it_sideways() -> TestResult {
    let (store, _) = with_note(&wide_table(), MARKDOWN)?;
    let mut harness = cards(&store);
    let table = harness
        .query_all_by_role_and_label(Role::Button, "Open the table")
        .next()
        .ok_or("the table")?
        .rect();
    let last = cell(&harness, "reading 8 of the year")?;
    // The bar lies right under the table; its thumb at its start.
    let grab = egui::pos2(table.left() + 20.0, table.bottom() + 6.0);
    harness.drag_at(grab);
    harness.run();
    harness.hover_at(grab + egui::vec2(200.0, 0.0));
    harness.run();
    harness.drop_at(grab + egui::vec2(200.0, 0.0));
    harness.run();
    let moved = cell(&harness, "reading 8 of the year")?;
    assert!(moved.left() < last.left() - 100.0, "{moved:?} {last:?}");
    // The card was not moved or opened.
    assert_eq!(harness.state().shown_table(), None);
    Ok(())
}

#[test]
fn a_table_that_fits_draws_its_heading_column_whole() -> TestResult {
    let text = "<!-- first column headings -->\n| Meter and what it counts in the house | Jan |\n\
                | --- | --- |\n| Water in cubic metres for the whole house | 312 |";
    let (store, _) = with_note(text, MARKDOWN)?;
    let harness = cards(&store);
    assert!(drawn_whole(
        &harness,
        "Water in cubic metres for the whole house"
    ));
    assert!(drawn_whole(&harness, "312"));
    Ok(())
}

#[test]
fn a_search_hit_can_unfold_a_cards_table_to_its_row() -> TestResult {
    // The hook for obf-1047: unfolded under the card's id, the row
    // brought into view.
    let (store, note) = with_note(&long_table(200), MARKDOWN)?;
    let mut harness = cards(&store);
    assert!(cell(&harness, "row 150").is_err());
    let of = egui::Id::new(("card", note.as_str()));
    oldbrainfix_gui::text_marks_view::unfold_table(&harness.ctx, of, 0, 150);
    harness.run();
    let row = cell(&harness, "row 150")?;
    assert!(row.top() >= 0.0 && row.bottom() <= 1000.0, "{row:?}");
    assert_eq!(buttons(&harness, "Show all 200"), 0);
    Ok(())
}
