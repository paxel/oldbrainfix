use super::*;

#[test]
fn every_cell_touching_the_region_holds_one_mark_inside_it() {
    let marks = question_marks_in(-10.0, -10.0, 310.0, 160.0, SEED);
    // Columns -1..=2, rows -1..=1.
    assert_eq!(marks.len(), 12);
    for m in &marks {
        assert!(m.shape < SHAPES.len());
        assert!((MIN_SIZE..MAX_SIZE).contains(&m.size));
        assert!(m.rotation.abs() <= MAX_TURN);
        let col = (m.x / CELL).floor();
        assert!(m.x - m.size / 2.0 >= col * CELL);
        assert!(m.x + m.size / 2.0 <= (col + 1.0) * CELL);
    }
    assert_eq!(marks, question_marks_in(-10.0, -10.0, 310.0, 160.0, SEED));
    assert_ne!(marks, question_marks_in(-10.0, -10.0, 310.0, 160.0, 1));
}
