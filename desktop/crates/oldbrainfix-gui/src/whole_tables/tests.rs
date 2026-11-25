//! The cases of the phone's `whole_tables_test.dart`, the same answers.

use super::*;

const A: &str = "| a |\n| --- |\n| 1 |";
const B: &str = "| b |\n| --- |";

/// The text with the table `table` of `before` moved; the change selects
/// the table moved.
fn moved(before: &str, table: &str, up: bool) -> String {
    let at = before.find(table).unwrap_or(usize::MAX);
    let Some(change) = move_table(before, at, up) else {
        panic!("no move of {table:?} in {before:?}");
    };
    assert_eq!(change.text.get(change.start..change.end), Some(table));
    change.text
}

#[test]
fn lists_paragraphs_and_quotes_run_on_and_blank_lines_part_the_blocks() {
    let text = format!(
        "# Head\nwords\nmore\n\n- one\n1. two\n  - three\n> said\n> on\n{A}\n\n```\n| x |\n| --- |\n```\n  \nend"
    );
    let found: Vec<(usize, usize, BlockKind)> = blocks(&text)
        .iter()
        .map(|b| (b.first, b.last, b.kind))
        .collect();
    assert_eq!(
        found,
        vec![
            (0, 0, BlockKind::Heading1),
            (1, 2, BlockKind::Paragraph),
            (4, 6, BlockKind::Bullet),
            (7, 8, BlockKind::Quote),
            (9, 11, BlockKind::Table),
            (13, 16, BlockKind::Code),
            (18, 18, BlockKind::Paragraph),
        ]
    );
}

#[test]
fn a_table_moves_past_a_paragraph_the_blank_lines_staying() {
    let text = format!("Shop\n\n{A}\n\nBread");
    assert_eq!(moved(&text, A, true), format!("{A}\n\nShop\n\nBread"));
    assert_eq!(moved(&text, A, false), format!("Shop\n\nBread\n\n{A}"));
    // Right above a paragraph, a blank line between.
    assert_eq!(
        moved(&format!("Shop\n{A}"), A, true),
        format!("{A}\n\nShop")
    );
}

#[test]
fn a_table_moves_past_another_also_one_right_under_it() {
    assert_eq!(
        moved(&format!("{A}\n\n{B}"), A, false),
        format!("{B}\n\n{A}")
    );
    let flagged = format!("{}\n{B}", table::HEADING_COLUMN_LINE);
    let text = moved(&format!("{A}\n{flagged}"), A, false);
    assert_eq!(text, format!("{flagged}\n\n{A}"));
    assert_eq!(table::all(&text).len(), 2);
    // A paragraph that comes right under another table gets a blank line
    // between.
    assert_eq!(
        moved(&format!("{A}\n{flagged}\n\npara"), &flagged, false),
        format!("{A}\n\npara\n\n{flagged}")
    );
    // One it comes right under gets a blank line between.
    let text = moved(&format!("{A}\n# H\n{B}"), B, true);
    assert_eq!(text, format!("{A}\n\n{B}\n# H"));
    assert_eq!(table::all(&text).len(), 2);
}

#[test]
fn a_code_block_and_a_list_are_one_block_each() {
    let code = "```\n| x |\n| --- |\n```";
    assert_eq!(
        moved(&format!("{code}\n\n{A}"), A, true),
        format!("{A}\n\n{code}")
    );
    let list = "- one\n- two\n  - three";
    assert_eq!(
        moved(&format!("{list}\n\n{A}\n\n# after"), A, true),
        format!("{A}\n\n{list}\n\n# after")
    );
}

#[test]
fn a_move_keeps_the_kind_of_line_break() {
    let crlf = "| a |\r\n| --- |";
    assert_eq!(
        moved(&format!("Shop\r\n\r\n{crlf}"), crlf, true),
        format!("{crlf}\r\n\r\nShop")
    );
    assert_eq!(
        moved(&format!("Shop\r\n{crlf}"), crlf, true),
        format!("{crlf}\r\n\r\nShop")
    );
}

#[test]
fn nothing_moves_past_the_top_or_the_bottom() {
    let text = format!("{A}\n\nBread");
    assert!(!can_move(&text, 0, true));
    assert_eq!(move_table(&text, 0, true), None);
    assert!(can_move(&text, 0, false));
    assert!(!can_move(&format!("Shop\n\n{A}\n\n"), 6, false));
    assert_eq!(move_table("Shop", 0, false), None);
    // As the ⋮ greys them.
    let first = |text: &str| table::all(text).into_iter().next();
    let table = first(&text).unwrap_or_else(|| panic!("a table"));
    assert_eq!(moves(&text, &table), (false, true));
    let text = format!("Shop\n\n{A}\n  \n");
    let table = first(&text).unwrap_or_else(|| panic!("a table"));
    assert_eq!(moves(&text, &table), (true, false));
}

#[test]
fn delete_takes_the_table_and_the_line_break_after_it() {
    let text = format!("Shop\n\n{A}\n\nBread");
    let deleted = delete_table(&text, 6).map(|c| c.text);
    assert_eq!(deleted.as_deref(), Some("Shop\n\n\nBread"));
    // As deleting its rows one by one does.
    let mut rows = text.clone();
    while !table::all(&rows).is_empty() {
        rows = table::delete_row(&rows, 6, 0).text;
    }
    assert_eq!(deleted, Some(rows));
}

#[test]
fn delete_of_the_last_table_in_the_text_takes_the_line_break_before_it() {
    let deleted = |text: &str, at: usize| delete_table(text, at).map(|c| c.text);
    assert_eq!(
        deleted(&format!("Shop\n\n{A}"), 6).as_deref(),
        Some("Shop\n")
    );
    assert_eq!(
        deleted("Shop\r\n| a |\r\n| --- |", 6).as_deref(),
        Some("Shop")
    );
    assert_eq!(deleted("Shop", 0), None);
}

#[test]
fn a_table_holds_words_while_a_cell_does() {
    assert!(holds_words(A, 0));
    assert!(!holds_words("|  |  |\n| --- | --- |", 0));
}
