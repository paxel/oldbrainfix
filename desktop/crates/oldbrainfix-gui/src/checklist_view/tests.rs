use super::*;

/// Rows 20 high, one under the other, a Line of a group after "  ".
fn rows(texts: &[&str]) -> Vec<DragRow> {
    texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let top = i as f32 * 20.0;
            let in_group = text.starts_with("  ");
            let left = if in_group { 20.0 } else { 0.0 };
            DragRow {
                id: text.trim().to_string(),
                in_group,
                rect: Rect::from_min_max(egui::pos2(left, top), egui::pos2(200.0, top + 20.0)),
            }
        })
        .collect()
}

/// Where `dragged` goes let go at the height `y`: before which Line,
/// `Some(None)` at the end of its level, `None` nowhere.
fn goes(rows: &[DragRow], dragged: &str, y: f32) -> Option<Option<String>> {
    line_drop(rows, dragged, egui::pos2(100.0, y)).map(|d| d.before)
}

#[test]
fn a_dragged_line_goes_into_a_gap_of_its_own_level() {
    let list = rows(&["A", "  a1", "  a2", "B", "C"]);
    // At the top, a Line with its group, the gaps between the Lines
    // at the top.
    assert_eq!(goes(&list, "C", 62.0), Some(Some("B".into())));
    assert_eq!(goes(&list, "C", 2.0), Some(Some("A".into())));
    assert_eq!(goes(&list, "A", 78.0), Some(Some("C".into())));
    assert_eq!(goes(&list, "A", 98.0), Some(None));
    // Over a group, the gap after it.
    assert_eq!(goes(&list, "C", 35.0), Some(Some("B".into())));
    // Within a group, among its Lines alone.
    assert_eq!(goes(&list, "a2", 22.0), Some(Some("a1".into())));
    assert_eq!(goes(&list, "a1", 58.0), Some(None));
    assert_eq!(goes(&list, "a1", 90.0), None);
    // Beside itself it stays.
    assert_eq!(goes(&list, "B", 58.0), None);
    assert_eq!(goes(&list, "B", 78.0), None);
    assert_eq!(goes(&list, "A", 2.0), None);
    // Away from the Lines too.
    assert_eq!(goes(&list, "C", 300.0), None);
    assert_eq!(line_drop(&list, "C", egui::pos2(500.0, 2.0)), None);
    assert_eq!(goes(&list, "gone", 2.0), None);
}
