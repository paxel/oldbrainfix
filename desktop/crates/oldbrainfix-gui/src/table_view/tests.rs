use super::*;

const TABLE: &str = "| Item | Qty |\n| --- | --- |\n| milk | 2 |";

fn view(text: &str, cell: Option<(usize, usize)>) -> TableView {
    TableView::new(Opening {
        note: Some("n".into()),
        text: text.into(),
        at: 0,
        cell,
        tags: true,
        boxes: Some("n".into()),
    })
}

#[test]
fn columns_are_lettered_as_a_sheet_letters_them() {
    assert_eq!(letter(0), "A");
    assert_eq!(letter(25), "Z");
    assert_eq!(letter(26), "AA");
    assert_eq!(letter(27), "AB");
    assert_eq!(letter(701), "ZZ");
    assert_eq!(letter(702), "AAA");
}

#[test]
fn a_cell_typed_into_is_written_and_one_left_alone_is_not() {
    let mut edited = view(TABLE, Some((1, 1)));
    assert_eq!(edited.commit(), None);
    let mut edited = view(TABLE, Some((1, 1)));
    if let Some(editing) = &mut edited.editing {
        editing.typed = "3 | 4".into();
    }
    let written = edited.commit();
    assert_eq!(
        written.as_deref(),
        Some("| Item | Qty |\n| --- | --- |\n| milk | 3 \\| 4 |")
    );
    assert_eq!(edited.editing, None);
}

#[test]
fn the_empty_row_and_column_add_one_only_with_words() {
    let mut blank = view(TABLE, Some((2, 0)));
    if let Some(editing) = &mut blank.editing {
        editing.typed = "  ".into();
    }
    assert_eq!(blank.commit(), None);
    let mut row = view(TABLE, Some((2, 0)));
    if let Some(editing) = &mut row.editing {
        editing.typed = "eggs".into();
    }
    assert_eq!(
        row.commit().as_deref(),
        Some("| Item | Qty |\n| --- | --- |\n| milk | 2 |\n| eggs |  |")
    );
    let mut column = view(TABLE, Some((0, 2)));
    if let Some(editing) = &mut column.editing {
        editing.typed = "Shop".into();
    }
    assert_eq!(
        column.commit().as_deref(),
        Some("| Item | Qty | Shop |\n| --- | --- | --- |\n| milk | 2 |  |")
    );
}

#[test]
fn deleting_the_last_row_takes_the_table_and_leaves() {
    let mut shown = view("| a |\n| --- |", None);
    let mut asked = Vec::new();
    shown.operate(Op::Delete, false, 0, &mut asked);
    // "a" is a word: asked first.
    assert_eq!(asked, []);
    let deleting = shown.deleting.take();
    assert_eq!(
        deleting,
        Some(Deleting {
            column: false,
            index: 0,
            gone: true
        })
    );
    if let Some(deleting) = deleting {
        shown.delete(deleting, &mut asked);
    }
    assert_eq!(asked, [Asked::Write(String::new()), Asked::Close]);
    // An empty row goes without asking.
    let mut empty = view("|  |  |\n| --- | --- |\n| x |  |", None);
    let mut asked = Vec::new();
    empty.operate(Op::Delete, true, 1, &mut asked);
    assert_eq!(asked, [Asked::Write("|  |\n| --- |\n| x |".into())]);
}

#[test]
fn tab_goes_row_by_row_into_the_empty_row() {
    let Some(table) = table::at(TABLE, 0) else {
        panic!("a table");
    };
    assert_eq!(next_cell(&table, (0, 0)), (0, 1));
    assert_eq!(next_cell(&table, (0, 1)), (1, 0));
    assert_eq!(next_cell(&table, (1, 1)), (2, 0));
    assert_eq!(next_cell(&table, (2, 1)), (2, 1));
    assert_eq!(previous_cell(&table, (1, 0)), (0, 1));
    assert_eq!(previous_cell(&table, (0, 0)), (0, 0));
}

#[test]
fn a_view_to_read_edits_nothing() {
    let mut read = TableView::new(Opening {
        note: None,
        text: TABLE.into(),
        at: 0,
        cell: Some((0, 0)),
        tags: false,
        boxes: None,
    });
    assert_eq!(read.editing, None);
    assert_eq!(read.commit(), None);
}
