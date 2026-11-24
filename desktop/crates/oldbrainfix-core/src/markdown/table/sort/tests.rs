use super::super::{all, at};
use super::*;

/// A table of one column holding `cells` under the header row `h`.
fn column(cells: &[&str]) -> String {
    let mut lines = vec!["| h |".to_string(), "|---|".to_string()];
    lines.extend(cells.iter().map(|c| format!("| {c} |")));
    lines.join("\n")
}

fn table_of(text: &str) -> Table {
    all(text)
        .first()
        .map(|t| Table::clone(t))
        .unwrap_or_else(|| panic!("a table in {text:?}"))
}

/// The cells of the one column of `cells` as [`sorted_rows`] orders them.
fn sorted(cells: &[&str], descending: bool) -> Vec<String> {
    let table = table_of(&column(cells));
    sorted_rows(&table, 0, descending)
        .into_iter()
        .map(|i| table.rows[i].cells[0].visible())
        .collect()
}

fn kind(cells: &[&str]) -> SortKind {
    sort_kind(&table_of(&column(cells)), 0)
}

#[test]
fn numbers_with_units_and_currency_signs_sort_by_number() {
    let cells = ["12,50 €", "3 kg", "€ 100", "-2", "−7 %", "0,5", "+4"];
    assert_eq!(kind(&cells), SortKind::Number);
    assert_eq!(
        sorted(&cells, false),
        ["−7 %", "-2", "0,5", "3 kg", "+4", "12,50 €", "€ 100"]
    );
}

#[test]
fn the_column_decides_the_decimal_mark() {
    assert_eq!(
        sorted(&["1.234", "12,5", "999"], false),
        ["12,5", "999", "1.234"]
    );
    assert_eq!(
        sorted(&["1,234", "1.5", "999"], false),
        ["1.5", "999", "1,234"]
    );
    assert_eq!(
        sorted(&["1.234.567", "2,345", "3"], false),
        ["2,345", "3", "1.234.567"]
    );
    assert_eq!(
        sorted(&["1,234", "567", "2,000"], false),
        ["567", "1,234", "2,000"]
    );
    assert_eq!(sorted(&["0.500", "2"], false), ["0.500", "2"]);
    assert_eq!(
        sorted(&["1'234.5", "1\u{a0}234.25 €", "999"], false),
        ["999", "1\u{a0}234.25 €", "1'234.5"]
    );
}

#[test]
fn cells_that_disagree_or_are_no_numbers_make_a_column_of_text() {
    for cells in [
        ["1,5", "2.5"],
        ["1.2,3", "4"],
        ["1,23,456", "4"],
        ["3 x 4", "5"],
        ["ca. 5", "6"],
        ["10:30", "9:15"],
        ["٣", "4"],
    ] {
        assert_eq!(kind(&cells), SortKind::Text, "{cells:?}");
    }
}

#[test]
fn numbers_compare_exactly_however_long() {
    assert_eq!(
        sorted(
            &[
                "12345678901234567890123",
                "12345678901234567890122.9",
                "-0",
                "0.0",
                "-0.000001",
            ],
            false
        ),
        [
            "-0.000001",
            "-0",
            "0.0",
            "12345678901234567890122.9",
            "12345678901234567890123",
        ]
    );
}

#[test]
fn dates_in_the_apps_formats_sort_by_date() {
    let cells = ["24.11.2026", "2026-01-05", "3/1/2026", "29.2.2024"];
    assert_eq!(kind(&cells), SortKind::Date);
    assert_eq!(
        sorted(&cells, false),
        ["29.2.2024", "2026-01-05", "3/1/2026", "24.11.2026"]
    );
    for cells in [
        ["31.02.2026", "1.1.2026"],
        ["29.2.2026", "1.1.2026"],
        ["24.11.26", "1.1.2026"],
        ["24/11/2026", "1/1/2026"],
        ["2026.11.24", "1.1.2026"],
    ] {
        assert_eq!(kind(&cells), SortKind::Text, "{cells:?}");
    }
    assert_eq!(kind(&["2026", "2025"]), SortKind::Number);
}

#[test]
fn text_ignores_case_accents_and_umlauts_and_keeps_equal_ones() {
    let cells = ["Zebra", "äpfel", "Müller", "muller", "Apfel", "eclair"];
    assert_eq!(kind(&cells), SortKind::Text);
    assert_eq!(
        sorted(&cells, false),
        ["äpfel", "Apfel", "eclair", "Müller", "muller", "Zebra"]
    );
    assert_eq!(
        sorted(&cells, true),
        ["Zebra", "Müller", "muller", "eclair", "äpfel", "Apfel"]
    );
    assert_eq!(sorted(&["🧠", "ａ"], false), ["ａ", "🧠"]);
    assert_eq!(sorted(&["**b**", "*a*", "`c`"], false), ["a", "b", "c"]);
}

#[test]
fn empty_cells_go_last_both_ways_short_rows_too() {
    let table = table_of("| a | b |\n|---|---|\n| x | 2 |\n| y |\n|  | 1 |\n| z | 3 |");
    assert_eq!(sorted_rows(&table, 1, false), [3, 1, 4, 2]);
    assert_eq!(sorted_rows(&table, 1, true), [4, 1, 3, 2]);
    assert_eq!(sorted_rows(&table, 0, false), [1, 2, 4, 3]);
    assert_eq!(sorted_rows(&table, 0, true), [4, 2, 1, 3]);
    assert_eq!(sorted_rows(&table, 2, false), [1, 2, 3, 4]);
    assert_eq!(kind(&["", ""]), SortKind::Text);
}

#[test]
fn equal_keys_keep_their_order_in_long_tables_both_ways() {
    let numbers: Vec<String> = (0..80).map(|i| format!("{} kg", i % 3)).collect();
    let refs: Vec<&str> = numbers.iter().map(String::as_str).collect();
    let table = table_of(&column(&refs));
    let of = |key: usize| (1..=80).filter(move |i| (i - 1) % 3 == key);
    let up: Vec<usize> = of(0).chain(of(1)).chain(of(2)).collect();
    let down: Vec<usize> = of(2).chain(of(1)).chain(of(0)).collect();
    assert_eq!(sorted_rows(&table, 0, false), up);
    assert_eq!(sorted_rows(&table, 0, true), down);
    let same: Vec<&str> = (0..80)
        .map(|i| if i % 2 == 0 { "Ä" } else { "a" })
        .collect();
    let letters = table_of(&column(&same));
    let written: Vec<usize> = (1..=80).collect();
    assert_eq!(sorted_rows(&letters, 0, false), written);
    assert_eq!(sorted_rows(&letters, 0, true), written);
}

#[test]
fn box_ids_and_variation_selectors_do_not_count() -> crate::Result<()> {
    // Ids that would sort the rows backwards.
    let mut ids = ["ccc", "bbb", "aaa"].into_iter();
    let text = crate::text_boxes::convert(
        &column(&["☐ b", "☐\u{fe0f} a", "☐ c"]),
        crate::text_format::MARKDOWN,
        || Ok(ids.next().unwrap_or("zzz").to_string()),
    )?;
    assert_eq!(crate::text_boxes::find(&text).len(), 3);
    assert_eq!(sorted_rows(&table_of(&text), 0, false), [2, 1, 3]);
    Ok(())
}

#[test]
fn sort_rows_moves_the_rows_lines_as_written_the_rest_untouched() {
    let head = format!("before\r\n{HEADING}\r\nItem|  Price\r\n:--- | ---:\r\n");
    let text =
        format!("{head}  bread | 2,50 € | beyond\r\n| milk |1 €|\r\neggs | 12 €\r\n\r\nafter");
    let at_item = text.find("Item").unwrap_or_default();
    let up = sort_rows(&text, at_item, 1, false);
    assert_eq!(
        up.text,
        format!("{head}| milk |1 €|\r\n  bread | 2,50 € | beyond\r\neggs | 12 €\r\n\r\nafter")
    );
    let table = at(&up.text, at_item).map(|t| (t.start, t.end));
    assert_eq!(Some((up.start, up.end)), table);
    let down = sort_rows(&text, at_item, 1, true);
    assert_eq!(
        down.text,
        format!("{head}eggs | 12 €\r\n  bread | 2,50 € | beyond\r\n| milk |1 €|\r\n\r\nafter")
    );
}

const HEADING: &str = super::super::HEADING_COLUMN_LINE;

#[test]
fn rows_already_in_order_a_missing_column_or_table_change_nothing() {
    let text = "x\n| a |\n|---|\n| 1 |\n| 2 |";
    for (column, descending) in [(0, false), (1, false), (9, true)] {
        let c = sort_rows(text, 4, column, descending);
        assert_eq!((c.text.as_str(), c.start, c.end), (text, 4, 4));
    }
    assert_eq!(
        sort_rows(text, 4, 0, true).text,
        "x\n| a |\n|---|\n| 2 |\n| 1 |"
    );
    let none = sort_rows("plain", 99, 0, false);
    assert_eq!((none.text.as_str(), none.start, none.end), ("plain", 5, 5));
}
