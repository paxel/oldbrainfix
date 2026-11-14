use super::*;

fn rows(text: &str) -> Vec<Vec<String>> {
    all(text)
        .first()
        .map(|t| {
            t.rows
                .iter()
                .map(|r| r.cells.iter().map(Cell::visible).collect())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_table_is_a_header_row_a_delimiter_row_and_the_rows_under() {
    let text = "Shop\n| Item | **Qty** |\n|:--|--:|\n| milk |\n| eggs | 6 | beyond |\nbread | `a\\|b` \\| c\n\nafter";
    let blocks = super::super::parse(text);
    let kinds: Vec<&str> = blocks.iter().map(|b| b.kind.name()).collect();
    assert_eq!(
        kinds,
        [
            "paragraph",
            "table",
            "table",
            "table",
            "table",
            "table",
            "paragraph",
            "paragraph"
        ]
    );
    let rows_of: Vec<Option<usize>> = blocks.iter().map(|b| b.row).collect();
    assert_eq!(
        rows_of,
        [None, Some(0), None, Some(1), Some(2), Some(3), None, None]
    );
    assert_eq!(
        rows(text),
        [
            vec!["Item", "Qty"],
            vec!["milk", ""],
            vec!["eggs", "6"],
            vec!["bread", "a|b | c"],
        ]
    );
    let table = all(text).first().cloned().ok_or("a table");
    assert_eq!(
        table.map(|t| t.aligns.clone()),
        Ok(vec![Align::Left, Align::Right])
    );
    // As GFM splits a row, a `\` before a pipe always keeps it in.
    assert_eq!(rows("a \\\\| b | c\n-|-"), [vec!["a \\| b", "c"]]);
    for none in [
        "a | b\n---",
        "a | b\n- | -",
        "# a | b\n--|--",
        "a | b\n|--|--|--|",
    ] {
        assert!(all(none).is_empty(), "{none:?}");
    }
}

#[test]
fn a_cell_is_written_so_that_it_stays_one_cell_of_one_line() {
    let text = "Before\n\n| a | b \\| c |\n|---|--:|\n| d |\n\nAfter";
    let at = text.find("| a").unwrap_or_default();
    assert_eq!(cell(text, at, 0, 1).as_deref(), Some("b | c"));
    assert_eq!(cell(text, at, 2, 0), None);
    for (value, written) in [
        ("# x", "# x"),
        ("- x", "- x"),
        ("a|b", "a\\|b"),
        ("`a|b`", "`a\\|b`"),
        ("a\r\nb\nc\rd", "a b c d"),
        ("  **x**\t", "**x**"),
    ] {
        let change = set_cell(text, at, 1, 1, value);
        assert_eq!(
            change.text,
            format!("Before\n\n| a | b \\| c |\n| --- | --: |\n| d | {written} |\n\nAfter"),
            "{value:?}"
        );
        assert_eq!(all(&change.text).first().map(|t| t.rows.len()), Some(2));
    }
    assert_eq!(set_cell(text, at, 2, 0, "x").text, text);
}

#[test]
fn a_column_lines_up_as_it_is_set_to() {
    let text = "x\n| a | b |\n|:-|-:|\n| c | d | e |\n\ny";
    let at = 2;
    let table = |c: Change| c.text[c.start..c.end].to_string();
    assert_eq!(
        table(set_align(text, at, 0, Align::Center)),
        "| a | b |\n| :-: | --: |\n| c | d | e |"
    );
    assert_eq!(
        table(set_align(text, at, 1, Align::None)),
        "| a | b |\n| :-- | --- |\n| c | d | e |"
    );
    let right = set_align(text, at, 0, Align::Right);
    assert_eq!(
        all(&right.text).first().map(|t| t.aligns.clone()),
        Some(vec![Align::Right, Align::Right])
    );
    assert_eq!(&right.text[..right.start], "x\n");
    assert_eq!(&right.text[right.end..], "\n\ny");
    // What cannot be done, or is so already, changes nothing.
    assert_eq!(set_align(text, at, 0, Align::Left).text, text);
    assert_eq!(set_align(text, at, 2, Align::Left).text, text);
    assert_eq!(set_align(text, 0, 0, Align::Center).text, text);
}

#[test]
fn the_last_row_or_column_going_takes_the_table_with_it() {
    let text = "x\n| a |\n|---|\n\ny";
    assert_eq!(delete_row(text, 2, 0).text, "x\n\ny");
    let last = delete_column("x\r\n| a |\r\n|-|", 4, 0);
    assert_eq!((last.text.as_str(), last.start), ("x", 1));
    // Never a panic, whatever the offsets.
    assert_eq!(insert_row("ü | b\n-|-", 1, 9).text, "ü | b\n-|-");
    assert_eq!(insert("ü", 2).text, "ü\n\n|  |  |\n| --- | --- |\n|  |  |");
}
