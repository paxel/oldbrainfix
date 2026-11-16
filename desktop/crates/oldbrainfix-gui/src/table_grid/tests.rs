use super::*;

const TABLE: &str = "| a | b |\n| --- | --- |\n| 1 | 2 |";

fn first(text: &str) -> Arc<Table> {
    match table::all(text).into_iter().next() {
        Some(table) => table,
        None => panic!("a table in {text:?}"),
    }
}

#[test]
fn what_is_typed_or_deleted_around_a_table_leaves_it_whole() {
    // Typed right under it: a blank line between.
    let under = format!("{TABLE}\n");
    let typed = format!("{TABLE}\nü");
    assert_eq!(
        keep_tables(&under, &typed, &[]),
        Some((format!("{TABLE}\n\nü"), TABLE.len() + 2 + "ü".len()))
    );
    // The blank line deleted that keeps a paragraph out: not deleted.
    let apart = format!("{TABLE}\n\nBread");
    let joined = format!("{TABLE}\nBread");
    assert_eq!(
        keep_tables(&apart, &joined, &[]),
        Some((apart.clone(), TABLE.len() + 1))
    );
    // Typed elsewhere, or by a table shown as lines: as typed.
    assert_eq!(keep_tables(&apart, &format!("{apart}s"), &[]), None);
    assert_eq!(keep_tables(&under, &typed, &[0]), None);
}

#[test]
fn leaving_after_a_table_lands_where_typing_joins_no_row() {
    // At the end of the text: a blank line and one to type on.
    let (text, at) = leave_after(TABLE, &first(TABLE));
    assert_eq!(text, format!("{TABLE}\n\n"));
    assert_eq!(at, text.len());
    // A blank line after it and words: at the start of the words.
    let after = format!("{TABLE}\n\nShop");
    let (text, at) = leave_after(&after, &first(&after));
    assert_eq!(text, after);
    assert_eq!(text.get(at..), Some("Shop"));
    // A heading right under it: at its start, nothing put in.
    let heading = format!("{TABLE}\n# Shop");
    let (text, at) = leave_after(&heading, &first(&heading));
    assert_eq!(text, heading);
    assert_eq!(text.get(at..), Some("# Shop"));
    // Only a line break after it: one more.
    let broken = format!("{TABLE}\n");
    let (text, at) = leave_after(&broken, &first(&broken));
    assert_eq!(text, format!("{TABLE}\n\n"));
    assert_eq!(at, text.len());
    assert_eq!(table::all(&format!("{}x", text)).len(), 1);
    assert_eq!(first(&format!("{}x", text)).rows.len(), 2);
}

#[test]
fn leaving_before_a_table_lands_at_the_end_of_the_line_above() {
    let (text, at) = leave_before(TABLE, &first(TABLE));
    assert_eq!(text, format!("\n{TABLE}"));
    assert_eq!(at, 0);
    let above = format!("Shop\n{TABLE}");
    let (text, at) = leave_before(&above, &first(&above));
    assert_eq!(text, above);
    assert_eq!(text.get(..at), Some("Shop"));
    // Two tables with a blank line between: a line put in, so what is
    // typed is no row of the one above.
    let two = format!("{TABLE}\n\n{TABLE}");
    let lower = match table::all(&two).get(1) {
        Some(t) => t.clone(),
        None => panic!("two tables"),
    };
    let (text, at) = leave_before(&two, &lower);
    assert_eq!(text, format!("{TABLE}\n\n\n{TABLE}"));
    let typed = format!("{}x{}", &text[..at], &text[at..]);
    assert_eq!(table::all(&typed).len(), 2);
    assert_eq!(first(&typed).rows.len(), 2);
}

#[test]
fn a_cells_words_map_to_the_text_as_written() {
    // "a | b" written "a \| b" from the byte 2 of the text.
    assert_eq!(text_byte(" a | b", 2, 1), 2);
    assert_eq!(text_byte(" a | b", 2, 2), 3);
    assert_eq!(text_byte(" a | b", 2, 6), 8);
    let text = "| x \\| y | 2 |\n| --- | --- |";
    let table = first(text);
    assert_eq!(cell_of(text, &table, 3), (0, 0));
    assert_eq!(cell_of(text, &table, 12), (0, 1));
    // The header row's first cell for the delimiter row.
    assert_eq!(cell_of(text, &table, 20), (0, 0));
    // "x \| y" reads "x | y", its "y" the fifth character.
    assert_eq!(draft_char(text, &table, (0, 0), 7), 4);
}

#[test]
fn a_tables_lines_are_hidden_and_its_first_as_high_as_the_grid() {
    let text = format!("Shop\n{TABLE}\n\nend");
    let mut job = LayoutJob::default();
    job.append(
        &text,
        0.0,
        TextFormat::simple(FontId::proportional(14.0), Color32::WHITE),
    );
    let table = first(&text);
    hide_range(&mut job, (table.start, table.start + 1, table.end), 90.0);
    let shown: Vec<_> = job
        .sections
        .iter()
        .filter(|s| s.format.color == Color32::WHITE)
        .filter_map(|s| text.get(s.byte_range.start.0..s.byte_range.end.0))
        .collect();
    assert_eq!(shown, ["Shop\n", "\n\nend"]);
    let high: Vec<_> = job
        .sections
        .iter()
        .filter(|s| s.format.line_height == Some(90.0))
        .filter_map(|s| text.get(s.byte_range.start.0..s.byte_range.end.0))
        .collect();
    assert_eq!(high, ["|"]);
}

#[test]
fn keys_step_row_by_row_and_out_at_the_edges() {
    let table = first(TABLE);
    let at = |step: Step, cell| match step.from(&table, cell) {
        Move::Cell(cell, _) => format!("{cell:?}"),
        Move::Out(after) => format!("out {after}"),
        Move::Field => "field".to_string(),
    };
    assert_eq!(at(Step::Next, (0, 1)), "(1, 0)");
    assert_eq!(at(Step::Next, (1, 1)), "out true");
    assert_eq!(at(Step::Previous, (1, 0)), "(0, 1)");
    assert_eq!(at(Step::Previous, (0, 0)), "out false");
    assert_eq!(at(Step::Below, (0, 1)), "(1, 1)");
    assert_eq!(at(Step::Below, (1, 1)), "out true");
    assert_eq!(at(Step::Above, (0, 0)), "out false");
    assert_eq!(at(Step::On, (0, 0)), "(0, 1)");
    assert_eq!(at(Step::Back, (0, 0)), "out false");
}
