//! The cases of the phone's `cell_selection_test.dart` for the cells as
//! a table of their own, the same answers.

use super::*;

const TABLE: &str = "| Item | Qty |\n| --- | --: |\n| ☐ milk | 2 |\n| oat | 1 |";

fn shop() -> String {
    format!("Shop\n\n{TABLE}\n\nBread")
}

#[test]
fn all_of_them_copy_as_the_whole_table_does() {
    let headings = format!(
        "{}\n| a | b \\| c |\n| :-: | --- |\n| x | y |",
        table::HEADING_COLUMN_LINE
    );
    for text in [shop(), headings] {
        let Some(start) = table::all(&text).first().map(|t| t.start) else {
            panic!("no table in {text:?}");
        };
        let none = BTreeMap::new();
        let copied = copy(&text, start, (&[0, 1, 2], (0, 1)), &none);
        let whole = table::copy(&text, start, &none);
        assert!(copied.is_some());
        assert_eq!(copied, whole);
    }
}

#[test]
fn the_rows_in_the_order_given_the_first_the_header_row_each_column_lining_up_as_it_does() {
    let text = shop();
    let at = text.find('|').unwrap_or_default();
    assert_eq!(
        table_cells(&text, at, &[2, 1], (1, 1)).as_deref(),
        Some("| 1 |\n| --: |\n| 2 |")
    );
    assert_eq!(
        table_cells(&text, at, &[1], (0, 1)).as_deref(),
        Some("| ☐ milk | 2 |\n| --- | --: |")
    );
}

#[test]
fn the_first_column_holds_headings_only_while_it_is_copied() {
    let line = table::HEADING_COLUMN_LINE;
    let text = format!("{line}\n| a | b |\n| --- | --- |\n| x | y |");
    assert_eq!(
        table_cells(&text, 0, &[0, 1], (0, 0)),
        Some(format!("{line}\n| a |\n| --- |\n| x |"))
    );
    assert_eq!(
        table_cells(&text, 0, &[0, 1], (1, 1)).as_deref(),
        Some("| b |\n| --- |\n| y |")
    );
}

#[test]
fn a_single_cell_copies_as_a_table_of_one_cell() {
    let text = shop();
    let at = text.find('|').unwrap_or_default();
    let copied = copy(&text, at, (&[2], (0, 0)), &BTreeMap::new());
    assert_eq!(
        copied.map(|c| c.markdown).as_deref(),
        Some("| oat |\n| --- |")
    );
}

#[test]
fn each_box_copies_in_the_state_it_shows() {
    let stored = oldbrainfix_core::text_boxes::convert(
        "| Item |\n| --- |\n| ☐ milk |",
        oldbrainfix_core::text_format::MARKDOWN,
        || Ok("b1b1".to_string()),
    );
    let Ok(text) = stored else {
        panic!("{stored:?}");
    };
    let Some(id) = oldbrainfix_core::text_boxes::find(&text)
        .first()
        .map(|b| b.id.clone())
    else {
        panic!("no box in {text:?}");
    };
    let states = BTreeMap::from([(id, BoxState::Done)]);
    let plain = copy(&text, 0, (&[0, 1], (0, 0)), &BTreeMap::new()).map(|c| c.markdown);
    let ticked = copy(&text, 0, (&[0, 1], (0, 0)), &states).map(|c| c.markdown);
    assert_eq!(plain.as_deref(), Some("| Item |\n| --- |\n| ☐ milk |"));
    assert_eq!(ticked.as_deref(), Some("| Item |\n| --- |\n| ✅ milk |"));
}

#[test]
fn none_where_there_is_no_table_or_no_cell() {
    let text = shop();
    let at = text.find('|').unwrap_or_default();
    assert_eq!(table_cells("Shop", 0, &[0], (0, 0)), None);
    assert_eq!(table_cells(&text, at, &[], (0, 0)), None);
    assert_eq!(table_cells(&text, at, &[9], (0, 0)), None);
    assert_eq!(table_cells(&text, at, &[0], (2, 3)), None);
}

#[test]
fn a_span_holds_the_cells_from_one_corner_to_the_other() {
    let span = Span {
        from: (3, 2),
        to: (1, 0),
    };
    let cells = span.cells();
    assert_eq!(
        cells,
        CellRect {
            top: 1,
            left: 0,
            bottom: 3,
            right: 2
        }
    );
    assert!(cells.holds((2, 1)));
    assert!(!cells.holds((0, 1)));
    assert!(!cells.one());
    assert!(span.within((4, 3)));
    assert!(!span.within((3, 3)));
}
